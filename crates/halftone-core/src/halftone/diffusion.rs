//! Floyd-Steinberg error diffusion on a cell grid.
//!
//! Diffusing at cell resolution keeps the SVG shape count manageable while
//! preserving the organic look of error diffusion. Use `cell = 1` for
//! pixel-level fidelity.

use crate::coverage::CoverageMap;
use crate::shape::Geometry;

/// Render the diffusion pattern.
pub fn render(map: &CoverageMap, cell: f32) -> Vec<Geometry> {
    let cell = cell.max(1.0);
    let cols = (map.width as f32 / cell).ceil().max(1.0) as u32;
    let rows = (map.height as f32 / cell).ceil().max(1.0) as u32;
    let mut values = map.block_average(cell, cols, rows);
    let mut out = Vec::new();
    let mut reverse = false;

    for j in 0..rows {
        let dir: i32 = if reverse { -1 } else { 1 };
        let start = if reverse { cols as i32 - 1 } else { 0 };

        for step in 0..cols as i32 {
            let i = start + dir * step;
            let idx = j as usize * cols as usize + i as usize;
            let value = values[idx].clamp(0.0, 1.0);
            let on = value > 0.5;
            let error = value - if on { 1.0 } else { 0.0 };

            if on {
                let x = i as f32 * cell;
                let y = j as f32 * cell;
                let w = cell.min(map.width as f32 - x);
                let h = cell.min(map.height as f32 - y);
                if w > 0.0 && h > 0.0 {
                    out.push(Geometry::Rect { x, y, w, h });
                }
            }

            let mut diffuse = |di: i32, dj: i32, weight: f32| {
                let ni = i + di;
                let nj = j as i32 + dj;
                if ni >= 0 && ni < cols as i32 && nj < rows as i32 {
                    let nidx = nj as usize * cols as usize + ni as usize;
                    values[nidx] += error * weight;
                }
            };
            diffuse(dir, 0, 7.0 / 16.0);
            diffuse(-dir, 1, 3.0 / 16.0);
            diffuse(0, 1, 5.0 / 16.0);
            diffuse(dir, 1, 1.0 / 16.0);
        }
        reverse = !reverse;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_fills_every_cell() {
        let map = CoverageMap::new(8, 8, vec![1.0; 64]);
        // Uniform full coverage can produce slightly fewer than all cells
        // because diffused error pushes later values above 1, never below the
        // threshold.
        assert_eq!(render(&map, 2.0).len(), 16);
    }

    #[test]
    fn white_produces_nothing() {
        let map = CoverageMap::new(8, 8, vec![0.0; 64]);
        assert!(render(&map, 2.0).is_empty());
    }

    #[test]
    fn mid_gray_produces_roughly_half() {
        let map = CoverageMap::new(16, 16, vec![0.5; 256]);
        let shapes = render(&map, 2.0);
        let total = 8 * 8;
        let ratio = shapes.len() as f32 / total as f32;
        assert!((0.35..=0.65).contains(&ratio), "ratio was {ratio}");
    }
}
