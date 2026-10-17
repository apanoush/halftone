//! Amplitude-modulated dot halftone.

use std::f32::consts::PI;

use crate::coverage::CoverageMap;
use crate::settings::DotShape;
use crate::shape::Geometry;

use super::grid::CellGrid;

/// Render one AM dot per cell.
pub fn render(
    map: &CoverageMap,
    cell: f32,
    angle_deg: f32,
    shape: DotShape,
    max_scale: f32,
) -> Vec<Geometry> {
    let grid = CellGrid::new(map, cell, angle_deg);
    let cell = grid.cell;
    let scale = max_scale.max(0.01);
    let mut out = Vec::new();
    for c in grid.cells() {
        let coverage = c.coverage.clamp(0.0, 1.0);
        if coverage < 0.005 {
            continue;
        }
        out.push(dot(shape, c.center, cell, coverage, angle_deg, scale));
    }
    out
}

/// Build a single dot centered on a cell.
fn dot(
    shape: DotShape,
    center: [f32; 2],
    cell: f32,
    coverage: f32,
    angle_deg: f32,
    scale: f32,
) -> Geometry {
    let angle = angle_deg.to_radians();
    let (sin, cos) = angle.sin_cos();
    let cell = cell * scale;

    match shape {
        DotShape::Circle => Geometry::Circle {
            cx: center[0],
            cy: center[1],
            r: (cell * (coverage / PI).sqrt()).max(0.05),
        },
        DotShape::Ellipse => {
            let aspect = 0.62f32;
            let rx = (cell * (coverage / (PI * aspect)).sqrt()).max(0.05);
            Geometry::Ellipse {
                cx: center[0],
                cy: center[1],
                rx,
                ry: rx * aspect,
                rotation_deg: angle_deg,
            }
        }
        DotShape::Square => {
            let half = cell * coverage.sqrt() * 0.5;
            polygon(
                center,
                &[[-half, -half], [half, -half], [half, half], [-half, half]],
                cos,
                sin,
            )
        }
        DotShape::Diamond => {
            let half = cell * (2.0 * coverage).sqrt() * 0.5;
            polygon(
                center,
                &[[0.0, -half], [half, 0.0], [0.0, half], [-half, 0.0]],
                cos,
                sin,
            )
        }
        DotShape::Cross => {
            // Plus sign with arm thickness L/3. Area = 5/9 L^2.
            let length = cell * (1.8 * coverage).sqrt();
            let h = length * 0.5;
            let a = length / 6.0;
            polygon(
                center,
                &[
                    [-a, -h],
                    [a, -h],
                    [a, -a],
                    [h, -a],
                    [h, a],
                    [a, a],
                    [a, h],
                    [-a, h],
                    [-a, a],
                    [-h, a],
                    [-h, -a],
                    [-a, -a],
                ],
                cos,
                sin,
            )
        }
    }
}

fn polygon(center: [f32; 2], local: &[[f32; 2]], cos: f32, sin: f32) -> Geometry {
    Geometry::Polygon(
        local
            .iter()
            .map(|[x, y]| [center[0] + x * cos - y * sin, center[1] + x * sin + y * cos])
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_map(width: u32, height: u32) -> CoverageMap {
        CoverageMap::new(width, height, vec![1.0; width as usize * height as usize])
    }

    #[test]
    fn one_dot_per_cell_at_full_coverage() {
        let map = full_map(16, 16);
        let shapes = render(&map, 4.0, 0.0, DotShape::Circle, 1.0);
        assert_eq!(shapes.len(), 16);
        for shape in shapes {
            match shape {
                Geometry::Circle { r, .. } => {
                    assert!((r - 4.0 / PI.sqrt()).abs() < 1e-4);
                }
                other => panic!("expected circle, got {other:?}"),
            }
        }
    }

    #[test]
    fn empty_map_produces_no_shapes() {
        let map = CoverageMap::new(8, 8, vec![0.0; 64]);
        assert!(render(&map, 4.0, 45.0, DotShape::Circle, 1.0).is_empty());
    }

    #[test]
    fn every_shape_variant_is_valid() {
        let map = full_map(8, 8);
        for shape in [
            DotShape::Circle,
            DotShape::Square,
            DotShape::Diamond,
            DotShape::Ellipse,
            DotShape::Cross,
        ] {
            assert!(!render(&map, 4.0, 30.0, shape, 1.0).is_empty());
        }
    }
}
