//! Clustered ordered (Bayer) dithering.
//!
//! Each `cell x cell` block is compared against a threshold from a Bayer
//! matrix. A passing block becomes a filled rectangle. This is the vector
//! analogue of ordered dithering and keeps the shape count bounded by the
//! number of blocks.

use crate::coverage::CoverageMap;
use crate::shape::Geometry;

const BAYER4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

const BAYER8: [[u8; 8]; 8] = [
    [0, 32, 8, 40, 2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44, 4, 36, 14, 46, 6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [3, 35, 11, 43, 1, 33, 9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47, 7, 39, 13, 45, 5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
];

/// Render the ordered dither.
pub fn render(map: &CoverageMap, cell: f32, matrix_size: u8) -> Vec<Geometry> {
    let cell = cell.max(1.0);
    let cols = (map.width as f32 / cell).ceil().max(1.0) as u32;
    let rows = (map.height as f32 / cell).ceil().max(1.0) as u32;
    let use_eight = matrix_size != 4;
    let mut out = Vec::new();

    for j in 0..rows {
        for i in 0..cols {
            let x = i as f32 * cell;
            let y = j as f32 * cell;
            if x >= map.width as f32 || y >= map.height as f32 {
                continue;
            }
            let coverage = map.average(x, y, cell, cell);
            let threshold = if use_eight {
                (BAYER8[(j % 8) as usize][(i % 8) as usize] as f32 + 0.5) / 64.0
            } else {
                (BAYER4[(j % 4) as usize][(i % 4) as usize] as f32 + 0.5) / 16.0
            };
            if coverage > threshold {
                let w = cell.min(map.width as f32 - x);
                let h = cell.min(map.height as f32 - y);
                if w > 0.0 && h > 0.0 {
                    out.push(Geometry::Rect { x, y, w, h });
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_produces_nothing() {
        let map = CoverageMap::new(16, 16, vec![0.0; 256]);
        assert!(render(&map, 4.0, 8).is_empty());
    }

    #[test]
    fn black_produces_one_rect_per_block() {
        let map = CoverageMap::new(16, 16, vec![1.0; 256]);
        assert_eq!(render(&map, 4.0, 8).len(), 16);
    }

    #[test]
    fn matrix_size_changes_pattern() {
        let map = CoverageMap::new(16, 16, vec![0.4; 256]);
        let four = render(&map, 4.0, 4);
        let eight = render(&map, 4.0, 8);
        assert!(!four.is_empty());
        assert!(!eight.is_empty());
    }
}
