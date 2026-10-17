//! Ink coverage maps.
//!
//! Every halftone algorithm consumes a `CoverageMap` where `0.0` means no ink
//! and `1.0` means full ink. This keeps color handling (mono vs CMYK) separate
//! from pattern generation.

use rayon::prelude::*;

use crate::raster::Raster;
use crate::tone::Tone;

/// A single-channel coverage image.
#[derive(Clone, Debug, PartialEq)]
pub struct CoverageMap {
    pub width: u32,
    pub height: u32,
    pub data: Vec<f32>,
}

impl CoverageMap {
    pub fn new(width: u32, height: u32, data: Vec<f32>) -> Self {
        debug_assert_eq!(data.len(), width as usize * height as usize);
        Self {
            width,
            height,
            data,
        }
    }

    /// Mono coverage: `1 - adjusted luminance`.
    pub fn mono(raster: &Raster, tone: &Tone) -> Self {
        let count = raster.width as usize * raster.height as usize;
        let mut data = vec![0.0f32; count];
        let width = raster.width;
        data.par_iter_mut().enumerate().for_each(|(i, out)| {
            let x = i as u32 % width;
            let y = i as u32 / width;
            let luma = tone.adjust(raster.luminance(x, y));
            *out = (1.0 - luma).clamp(0.0, 1.0);
        });
        Self::new(raster.width, raster.height, data)
    }

    /// CMYK separation. Each returned map holds ink coverage for
    /// cyan, magenta, yellow, black, in that order.
    pub fn cmyk(raster: &Raster, tone: &Tone) -> [Self; 4] {
        let count = raster.width as usize * raster.height as usize;
        let width = raster.width;
        let height = raster.height;
        let channels: Vec<Vec<f32>> = (0..4)
            .into_par_iter()
            .map(|channel| {
                let mut data = vec![0.0f32; count];
                for y in 0..height {
                    for x in 0..width {
                        let [r, g, b] = raster.rgb_over_white(x, y);
                        let [c, m, yel, k] =
                            rgb_to_cmyk(tone.adjust(r), tone.adjust(g), tone.adjust(b));
                        let value = [c, m, yel, k][channel];
                        data[(y as usize * width as usize) + x as usize] = value.clamp(0.0, 1.0);
                    }
                }
                data
            })
            .collect();

        let mut iter = channels.into_iter();
        let c = Self::new(width, height, iter.next().expect("four channels"));
        let m = Self::new(width, height, iter.next().expect("four channels"));
        let y = Self::new(width, height, iter.next().expect("four channels"));
        let k = Self::new(width, height, iter.next().expect("four channels"));
        [c, m, y, k]
    }

    #[inline]
    pub fn get(&self, x: u32, y: u32) -> f32 {
        self.data[y as usize * self.width as usize + x as usize]
    }

    /// Mean coverage over a rectangular region, clamped to the image.
    pub fn average(&self, x: f32, y: f32, w: f32, h: f32) -> f32 {
        let x0 = x.max(0.0).floor() as u32;
        let y0 = y.max(0.0).floor() as u32;
        let x1 = (x + w).ceil().clamp(0.0, self.width as f32) as u32;
        let y1 = (y + h).ceil().clamp(0.0, self.height as f32) as u32;
        if x1 <= x0 || y1 <= y0 {
            return 0.0;
        }
        let mut sum = 0.0;
        let mut count = 0u32;
        for yy in y0..y1 {
            for xx in x0..x1 {
                sum += self.get(xx, yy);
                count += 1;
            }
        }
        if count == 0 { 0.0 } else { sum / count as f32 }
    }

    /// Average over a cell-aligned grid, used by ordered and diffusion modes.
    pub fn block_average(&self, cell: f32, cols: u32, rows: u32) -> Vec<f32> {
        let mut out = Vec::with_capacity(cols as usize * rows as usize);
        for j in 0..rows {
            for i in 0..cols {
                out.push(self.average(i as f32 * cell, j as f32 * cell, cell, cell));
            }
        }
        out
    }
}

/// Convert normalized RGB to ink amounts `[c, m, y, k]`.
pub fn rgb_to_cmyk(r: f32, g: f32, b: f32) -> [f32; 4] {
    let r = r.clamp(0.0, 1.0);
    let g = g.clamp(0.0, 1.0);
    let b = b.clamp(0.0, 1.0);
    let k = 1.0 - r.max(g).max(b);
    if k >= 1.0 - 1e-6 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let inv = 1.0 - k;
    [
        ((1.0 - r - k) / inv).clamp(0.0, 1.0),
        ((1.0 - g - k) / inv).clamp(0.0, 1.0),
        ((1.0 - b - k) / inv).clamp(0.0, 1.0),
        k.clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgba;

    #[test]
    fn mono_white_has_no_ink() {
        let raster = Raster::new(2, 2);
        let map = CoverageMap::mono(&raster, &Tone::identity());
        assert!(map.data.iter().all(|v| *v < 1e-6));
    }

    #[test]
    fn mono_black_is_full_ink() {
        let mut raster = Raster::new(2, 2);
        for y in 0..2 {
            for x in 0..2 {
                raster.set_pixel(x, y, Rgba::BLACK);
            }
        }
        let map = CoverageMap::mono(&raster, &Tone::identity());
        assert!(map.data.iter().all(|v| *v > 0.999));
    }

    #[test]
    fn cmyk_black_only_uses_k() {
        let [c, m, y, k] = rgb_to_cmyk(0.0, 0.0, 0.0);
        assert_eq!([c, m, y, k], [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn cmyk_cyan_pure() {
        let [c, m, y, k] = rgb_to_cmyk(0.0, 1.0, 1.0);
        assert!((c - 1.0).abs() < 1e-6);
        assert!(m.abs() < 1e-6 && y.abs() < 1e-6 && k.abs() < 1e-6);
    }

    #[test]
    fn average_is_clamped_to_bounds() {
        let map = CoverageMap::new(4, 4, vec![0.5; 16]);
        assert!((map.average(-10.0, -10.0, 100.0, 100.0) - 0.5).abs() < 1e-6);
        assert!(map.average(10.0, 10.0, 2.0, 2.0).abs() < 1e-6);
    }
}
