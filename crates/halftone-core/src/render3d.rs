//! Software rasterizer for 3D meshes.
//!
//! The renderer is intentionally small: perspective projection, a z-buffer,
//! and Lambert shading with an ambient term. Output is an RGBA raster that
//! feeds the same halftone pipeline as photographs. This keeps CLI and GUI
//! behavior identical and avoids any GPU dependency.

use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

use crate::color::Rgba;
use crate::mesh::Mesh;
use crate::raster::Raster;

/// Camera and lighting for mesh rendering.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Camera {
    /// Euler rotation in degrees, applied as Y then X then Z.
    pub rotation: [f32; 3],
    /// Camera zoom factor. `1.0` fits the normalized mesh.
    pub zoom: f32,
    /// Direction toward the light.
    pub light: [f32; 3],
    /// Ambient intensity in `0..=1`.
    pub ambient: f32,
    /// Vertical field of view in degrees.
    pub fov: f32,
    /// Diffuse base color of the surface.
    pub base_color: Rgba,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            rotation: [18.0, 30.0, 0.0],
            zoom: 1.0,
            light: [0.35, 0.7, 0.6],
            ambient: 0.18,
            fov: 45.0,
            base_color: Rgba::rgb(235, 235, 235),
        }
    }
}

/// Rasterize a mesh into an RGBA image.
pub fn rasterize(mesh: &Mesh, camera: &Camera, width: u32, height: u32) -> Raster {
    let width = width.max(1);
    let height = height.max(1);
    let mut raster = Raster::new(width, height);
    if mesh.indices.is_empty() {
        return raster;
    }

    let mesh = mesh.normalized();
    let (yaw, pitch, roll) = (
        camera.rotation[1].to_radians(),
        camera.rotation[0].to_radians(),
        camera.rotation[2].to_radians(),
    );
    let model =
        Mat4::from_rotation_y(yaw) * Mat4::from_rotation_x(pitch) * Mat4::from_rotation_z(roll);

    let distance = 3.0 / camera.zoom.max(0.05);
    let view =
        glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, 0.0, distance), Vec3::ZERO, Vec3::Y);
    let aspect = width as f32 / height as f32;
    let projection = glam::camera::rh::proj::opengl::perspective(
        camera.fov.clamp(10.0, 120.0).to_radians(),
        aspect,
        0.05,
        100.0,
    );
    let view_projection = projection * view;

    let light = Vec3::from_array(camera.light).normalize_or_zero();
    let light = if light.length_squared() < 1e-6 {
        Vec3::new(0.35, 0.7, 0.6).normalize()
    } else {
        light
    };
    let ambient = camera.ambient.clamp(0.0, 1.0);
    let base = camera.base_color;

    let mut depth = vec![f32::INFINITY; width as usize * height as usize];
    let mut world = Vec::with_capacity(mesh.positions.len());
    for p in &mesh.positions {
        world.push(model.transform_point3(Vec3::from_array(*p)));
    }

    for [i0, i1, i2] in &mesh.indices {
        let (Some(w0), Some(w1), Some(w2)) = (
            world.get(*i0 as usize),
            world.get(*i1 as usize),
            world.get(*i2 as usize),
        ) else {
            continue;
        };
        let clip0 = view_projection * w0.extend(1.0);
        let clip1 = view_projection * w1.extend(1.0);
        let clip2 = view_projection * w2.extend(1.0);
        if clip0.w <= 1e-4 || clip1.w <= 1e-4 || clip2.w <= 1e-4 {
            continue;
        }
        let s0 = to_screen(clip0, width, height);
        let s1 = to_screen(clip1, width, height);
        let s2 = to_screen(clip2, width, height);

        let area = (s1.x - s0.x) * (s2.y - s0.y) - (s2.x - s0.x) * (s1.y - s0.y);
        if area.abs() < 1e-6 {
            continue;
        }
        let inv_area = 1.0 / area;

        let min_x = s0.x.min(s1.x).min(s2.x).floor().max(0.0) as i32;
        let max_x = s0.x.max(s1.x).max(s2.x).ceil().min(width as f32 - 1.0) as i32;
        let min_y = s0.y.min(s1.y).min(s2.y).floor().max(0.0) as i32;
        let max_y = s0.y.max(s1.y).max(s2.y).ceil().min(height as f32 - 1.0) as i32;
        if max_x < min_x || max_y < min_y {
            continue;
        }

        let face_normal = (*w1 - *w0).cross(*w2 - *w0).normalize_or_zero();
        let diffuse = face_normal.dot(light).abs();
        let intensity = (ambient + (1.0 - ambient) * diffuse).clamp(0.0, 1.0);
        let shaded = Rgba::new(
            (base.r as f32 * intensity).clamp(0.0, 255.0) as u8,
            (base.g as f32 * intensity).clamp(0.0, 255.0) as u8,
            (base.b as f32 * intensity).clamp(0.0, 255.0) as u8,
            255,
        );

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let e0 = (s1.x - px) * (s2.y - py) - (s2.x - px) * (s1.y - py);
                let e1 = (s2.x - px) * (s0.y - py) - (s0.x - px) * (s2.y - py);
                let e2 = (s0.x - px) * (s1.y - py) - (s1.x - px) * (s0.y - py);
                let b0 = e0 * inv_area;
                let b1 = e1 * inv_area;
                let b2 = e2 * inv_area;
                if b0 < -1e-4 || b1 < -1e-4 || b2 < -1e-4 {
                    continue;
                }
                let z = b0 * s0.z + b1 * s1.z + b2 * s2.z;
                let index = y as usize * width as usize + x as usize;
                if z < depth[index] {
                    depth[index] = z;
                    raster.set_pixel(x as u32, y as u32, shaded);
                }
            }
        }
    }
    raster
}

fn to_screen(clip: glam::Vec4, width: u32, height: u32) -> Vec3 {
    let ndc = clip.truncate() / clip.w;
    Vec3::new(
        (ndc.x * 0.5 + 0.5) * width as f32,
        (0.5 - ndc.y * 0.5) * height as f32,
        ndc.z,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube() -> Mesh {
        let mut positions = Vec::new();
        for z in [-1.0f32, 1.0] {
            for y in [-1.0f32, 1.0] {
                for x in [-1.0f32, 1.0] {
                    positions.push([x, y, z]);
                }
            }
        }
        let indices = vec![
            [0, 1, 3],
            [4, 6, 7],
            [0, 4, 5],
            [2, 3, 7],
            [0, 2, 6],
            [1, 5, 7],
        ];
        Mesh { positions, indices }
    }

    #[test]
    fn rasterizes_a_cube() {
        let raster = rasterize(&cube(), &Camera::default(), 64, 64);
        let opaque = raster.data.chunks_exact(4).filter(|p| p[3] == 255).count();
        assert!(opaque > 100, "only {opaque} pixels were covered");
        assert!(opaque < 64 * 64);
    }

    #[test]
    fn empty_mesh_stays_transparent() {
        let raster = rasterize(&Mesh::default(), &Camera::default(), 16, 16);
        assert!(raster.data.iter().all(|b| *b == 0));
    }

    #[test]
    fn rotation_changes_output() {
        let a = rasterize(&cube(), &Camera::default(), 32, 32);
        let camera = Camera {
            rotation: [50.0, 120.0, 10.0],
            ..Camera::default()
        };
        let b = rasterize(&cube(), &camera, 32, 32);
        assert_ne!(a, b);
    }
}
