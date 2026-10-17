//! Triangle mesh loading and normalization.

use std::path::Path;

use crate::error::{Error, Result};

/// A triangle mesh with indexed positions.
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<[u32; 3]>,
}

impl Mesh {
    pub fn triangle_count(&self) -> usize {
        self.indices.len()
    }

    /// Center the mesh at the origin and scale it into a unit bounding sphere.
    ///
    /// Fitting the sphere (rather than the bounding box) guarantees the model
    /// stays inside the camera frustum at every rotation.
    pub fn normalized(&self) -> Self {
        if self.positions.is_empty() {
            return Self::default();
        }
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for p in &self.positions {
            for axis in 0..3 {
                min[axis] = min[axis].min(p[axis]);
                max[axis] = max[axis].max(p[axis]);
            }
        }
        let center = [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ];
        let radius = self
            .positions
            .iter()
            .map(|p| {
                let dx = p[0] - center[0];
                let dy = p[1] - center[1];
                let dz = p[2] - center[2];
                (dx * dx + dy * dy + dz * dz).sqrt()
            })
            .fold(0.0f32, f32::max)
            .max(1e-6);
        let scale = 1.0 / radius;
        let positions = self
            .positions
            .iter()
            .map(|p| {
                [
                    (p[0] - center[0]) * scale,
                    (p[1] - center[1]) * scale,
                    (p[2] - center[2]) * scale,
                ]
            })
            .collect();
        Self {
            positions,
            indices: self.indices.clone(),
        }
    }
}

/// Load an OBJ file, merging all models and triangulating faces.
pub fn load_obj(path: &Path) -> Result<Mesh> {
    let options = tobj::LoadOptions {
        triangulate: true,
        single_index: true,
        ignore_points: true,
        ignore_lines: true,
    };
    let (models, _materials) =
        tobj::load_obj(path, &options).map_err(|e| Error::Obj(e.to_string()))?;

    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for model in models {
        let mesh = &model.mesh;
        let offset = positions.len() as u32;
        positions.extend(mesh.positions.chunks_exact(3).map(|c| [c[0], c[1], c[2]]));
        indices.extend(
            mesh.indices
                .chunks_exact(3)
                .map(|c| [c[0] + offset, c[1] + offset, c[2] + offset]),
        );
    }

    if indices.is_empty() {
        return Err(Error::Obj("mesh contains no triangles".to_string()));
    }
    Ok(Mesh { positions, indices })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_cube() -> Mesh {
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        for z in [-1.0f32, 1.0] {
            for y in [-1.0f32, 1.0] {
                for x in [-1.0f32, 1.0] {
                    positions.push([x, y, z]);
                }
            }
        }
        // One triangle per face, enough to exercise the rasterizer.
        indices.extend([
            [0, 1, 3],
            [4, 6, 7],
            [0, 4, 5],
            [2, 3, 7],
            [0, 2, 6],
            [1, 5, 7],
        ]);
        Mesh { positions, indices }
    }

    #[test]
    fn normalization_fits_in_unit_box() {
        let mesh = Mesh {
            positions: vec![[0.0, 0.0, 0.0], [10.0, 5.0, 2.0]],
            indices: vec![[0, 1, 0]],
        };
        let n = mesh.normalized();
        for p in &n.positions {
            for v in p {
                assert!(v.abs() <= 1.0 + 1e-5);
            }
        }
    }

    #[test]
    fn cube_has_expected_triangle_count() {
        assert_eq!(unit_cube().triangle_count(), 6);
    }
}
