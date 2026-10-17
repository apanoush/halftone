//! Rotated cell and line accumulation grids.
//!
//! Both grids map every source pixel to a lattice coordinate, accumulate
//! coverage, and expose the average per cell. Working in the rotated lattice
//! keeps dots and lines anchored to the screen angle.

use crate::coverage::CoverageMap;

const MIN_CELL: f32 = 0.25;

/// A rotated square lattice of halftone cells.
pub struct CellGrid {
    pub cell: f32,
    origin: [f32; 2],
    cos: f32,
    sin: f32,
    imin: i32,
    jmin: i32,
    ni: usize,
    nj: usize,
    sums: Vec<f32>,
    counts: Vec<u32>,
}

/// One populated cell.
pub struct Cell {
    pub center: [f32; 2],
    pub coverage: f32,
}

impl CellGrid {
    pub fn new(map: &CoverageMap, cell: f32, angle_deg: f32) -> Self {
        let cell = cell.max(MIN_CELL);
        let angle = angle_deg.to_radians();
        let (sin, cos) = angle.sin_cos();
        let origin = [map.width as f32 * 0.5, map.height as f32 * 0.5];

        let corners = [
            [0.0f32, 0.0f32],
            [map.width as f32, 0.0],
            [0.0, map.height as f32],
            [map.width as f32, map.height as f32],
        ];
        let mut imin = i32::MAX;
        let mut imax = i32::MIN;
        let mut jmin = i32::MAX;
        let mut jmax = i32::MIN;
        for [x, y] in corners {
            let dx = x - origin[0];
            let dy = y - origin[1];
            let fi = (dx * cos + dy * sin) / cell;
            let fj = (-dx * sin + dy * cos) / cell;
            imin = imin.min(fi.floor() as i32 - 1);
            imax = imax.max(fi.floor() as i32 + 1);
            jmin = jmin.min(fj.floor() as i32 - 1);
            jmax = jmax.max(fj.floor() as i32 + 1);
        }
        let ni = (imax - imin + 1).max(1) as usize;
        let nj = (jmax - jmin + 1).max(1) as usize;

        let mut grid = Self {
            cell,
            origin,
            cos,
            sin,
            imin,
            jmin,
            ni,
            nj,
            sums: vec![0.0; ni * nj],
            counts: vec![0; ni * nj],
        };
        grid.accumulate(map);
        grid
    }

    fn accumulate(&mut self, map: &CoverageMap) {
        let width = map.width as usize;
        for y in 0..map.height {
            for x in 0..map.width {
                let dx = x as f32 + 0.5 - self.origin[0];
                let dy = y as f32 + 0.5 - self.origin[1];
                let fi = (dx * self.cos + dy * self.sin) / self.cell;
                let fj = (-dx * self.sin + dy * self.cos) / self.cell;
                let ii = fi.floor() as i32 - self.imin;
                let jj = fj.floor() as i32 - self.jmin;
                if ii < 0 || jj < 0 || ii as usize >= self.ni || jj as usize >= self.nj {
                    continue;
                }
                let idx = jj as usize * self.ni + ii as usize;
                self.sums[idx] += map.data[y as usize * width + x as usize];
                self.counts[idx] += 1;
            }
        }
    }

    /// Iterate over cells that received at least one source pixel.
    pub fn cells(&self) -> impl Iterator<Item = Cell> + '_ {
        let ni = self.ni;
        (0..self.nj).flat_map(move |j| {
            (0..ni).filter_map(move |i| {
                let idx = j * ni + i;
                let count = self.counts[idx];
                if count == 0 {
                    return None;
                }
                let fi = (self.imin + i as i32) as f32 + 0.5;
                let fj = (self.jmin + j as i32) as f32 + 0.5;
                let cx = self.origin[0] + (fi * self.cos - fj * self.sin) * self.cell;
                let cy = self.origin[1] + (fi * self.sin + fj * self.cos) * self.cell;
                let coverage = self.sums[idx] / count as f32;
                Some(Cell {
                    center: [cx, cy],
                    coverage,
                })
            })
        })
    }
}

/// Parallel line bands at one angle.
pub struct LineGrid {
    pub spacing: f32,
    pub origin: [f32; 2],
    pub cos: f32,
    pub sin: f32,
    kmin: i32,
    sums: Vec<f32>,
    counts: Vec<u32>,
}

impl LineGrid {
    pub fn new(map: &CoverageMap, spacing: f32, angle_deg: f32) -> Self {
        let spacing = spacing.max(MIN_CELL);
        let angle = angle_deg.to_radians();
        let (sin, cos) = angle.sin_cos();
        let origin = [map.width as f32 * 0.5, map.height as f32 * 0.5];

        let corners = [
            [0.0f32, 0.0f32],
            [map.width as f32, 0.0],
            [0.0, map.height as f32],
            [map.width as f32, map.height as f32],
        ];
        let mut kmin = i32::MAX;
        let mut kmax = i32::MIN;
        for [x, y] in corners {
            let dx = x - origin[0];
            let dy = y - origin[1];
            let t = -dx * sin + dy * cos;
            let k = (t / spacing).floor() as i32;
            kmin = kmin.min(k - 1);
            kmax = kmax.max(k + 1);
        }
        let count = (kmax - kmin + 1).max(1) as usize;
        let mut grid = Self {
            spacing,
            origin,
            cos,
            sin,
            kmin,
            sums: vec![0.0; count],
            counts: vec![0; count],
        };
        grid.accumulate(map);
        grid
    }

    fn accumulate(&mut self, map: &CoverageMap) {
        let width = map.width as usize;
        for y in 0..map.height {
            for x in 0..map.width {
                let dx = x as f32 + 0.5 - self.origin[0];
                let dy = y as f32 + 0.5 - self.origin[1];
                let t = -dx * self.sin + dy * self.cos;
                let idx = (t / self.spacing).floor() as i32 - self.kmin;
                if idx < 0 || idx as usize >= self.sums.len() {
                    continue;
                }
                let idx = idx as usize;
                self.sums[idx] += map.data[y as usize * width + x as usize];
                self.counts[idx] += 1;
            }
        }
    }

    /// Iterate over line bands as `(signed normal offset, mean coverage)`.
    pub fn lines(&self) -> impl Iterator<Item = (f32, f32)> + '_ {
        (0..self.sums.len()).filter_map(move |idx| {
            let count = self.counts[idx];
            if count == 0 {
                return None;
            }
            let k = self.kmin + idx as i32;
            let t = (k as f32 + 0.5) * self.spacing;
            Some((t, self.sums[idx] / count as f32))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_map(width: u32, height: u32) -> CoverageMap {
        CoverageMap::new(width, height, vec![1.0; width as usize * height as usize])
    }

    #[test]
    fn axis_aligned_cell_grid_has_expected_count() {
        let map = full_map(16, 16);
        let grid = CellGrid::new(&map, 4.0, 0.0);
        assert_eq!(grid.cells().count(), 16);
    }

    #[test]
    fn cell_centers_are_cell_centered() {
        let map = full_map(8, 8);
        let grid = CellGrid::new(&map, 4.0, 0.0);
        let centers: Vec<_> = grid.cells().map(|c| c.center).collect();
        assert!(centers.contains(&[2.0, 2.0]));
        assert!(centers.contains(&[6.0, 6.0]));
    }

    #[test]
    fn line_grid_reports_full_coverage() {
        let map = full_map(8, 8);
        let grid = LineGrid::new(&map, 2.0, 0.0);
        for (_, cov) in grid.lines() {
            assert!((cov - 1.0).abs() < 1e-6);
        }
    }
}
