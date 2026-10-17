//! Line and crosshatch halftones.

use crate::coverage::CoverageMap;
use crate::shape::Geometry;

use super::grid::LineGrid;

/// Parallel lines, one per band.
pub fn render(map: &CoverageMap, spacing: f32, angle_deg: f32, max_scale: f32) -> Vec<Geometry> {
    render_set(map, spacing, angle_deg, max_scale, |c| c)
}

/// Three line sets. The second and third only appear in darker areas.
pub fn render_crosshatch(
    map: &CoverageMap,
    spacing: f32,
    angle_deg: f32,
    max_scale: f32,
) -> Vec<Geometry> {
    let mut out = render_set(map, spacing, angle_deg, max_scale, |c| c);
    out.extend(render_set(map, spacing, angle_deg + 60.0, max_scale, |c| {
        (3.0 * c - 1.0).clamp(0.0, 1.0)
    }));
    out.extend(render_set(
        map,
        spacing,
        angle_deg + 120.0,
        max_scale,
        |c| (3.0 * c - 2.0).clamp(0.0, 1.0),
    ));
    out
}

fn render_set<F>(
    map: &CoverageMap,
    spacing: f32,
    angle_deg: f32,
    max_scale: f32,
    coverage_transform: F,
) -> Vec<Geometry>
where
    F: Fn(f32) -> f32,
{
    let grid = LineGrid::new(map, spacing, angle_deg);
    let spacing = grid.spacing;
    let (sin, cos) = (grid.sin, grid.cos);
    let origin = grid.origin;
    let diagonal = ((map.width as f32).powi(2) + (map.height as f32).powi(2)).sqrt();
    let half_len = diagonal * 0.5 + spacing;
    let scale = max_scale.max(0.01);

    let mut out = Vec::new();
    for (offset, mean_coverage) in grid.lines() {
        let width = spacing * coverage_transform(mean_coverage).clamp(0.0, 1.0) * scale;
        if width < 0.01 {
            continue;
        }
        // Center of this band: origin + normal * offset.
        let cx = origin[0] - sin * offset;
        let cy = origin[1] + cos * offset;
        // Along the band.
        let ux = cos;
        let uy = sin;
        // Normal to the band.
        let nx = -sin;
        let ny = cos;
        let hw = width * 0.5;
        out.push(Geometry::Polygon(vec![
            [cx - ux * half_len - nx * hw, cy - uy * half_len - ny * hw],
            [cx + ux * half_len - nx * hw, cy + uy * half_len - ny * hw],
            [cx + ux * half_len + nx * hw, cy + uy * half_len + ny * hw],
            [cx - ux * half_len + nx * hw, cy - uy * half_len + ny * hw],
        ]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_coverage_gives_one_solid_line_per_band() {
        let map = CoverageMap::new(8, 8, vec![1.0; 64]);
        let shapes = render(&map, 2.0, 0.0, 1.0);
        assert_eq!(shapes.len(), 4);
        for shape in shapes {
            match shape {
                Geometry::Polygon(points) => assert_eq!(points.len(), 4),
                other => panic!("expected polygon, got {other:?}"),
            }
        }
    }

    #[test]
    fn crosshatch_adds_sets_in_dark_areas() {
        let map = CoverageMap::new(16, 16, vec![1.0; 256]);
        let plain = render(&map, 4.0, 0.0, 1.0);
        let cross = render_crosshatch(&map, 4.0, 0.0, 1.0);
        assert!(cross.len() > plain.len());
    }

    #[test]
    fn empty_map_produces_no_lines() {
        let map = CoverageMap::new(8, 8, vec![0.0; 64]);
        assert!(render(&map, 2.0, 0.0, 1.0).is_empty());
    }
}
