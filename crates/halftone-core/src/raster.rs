//! In-memory RGBA8 raster.

use crate::color::Rgba;

/// An RGBA8 image buffer, row-major, 4 bytes per pixel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Raster {
    /// Create a fully transparent raster.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0; width as usize * height as usize * 4],
        }
    }

    pub fn from_rgba8(width: u32, height: u32, data: Vec<u8>) -> Self {
        debug_assert_eq!(data.len(), width as usize * height as usize * 4);
        Self {
            width,
            height,
            data,
        }
    }

    #[inline]
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = (y as usize * self.width as usize + x as usize) * 4;
        [
            self.data[i],
            self.data[i + 1],
            self.data[i + 2],
            self.data[i + 3],
        ]
    }

    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Rgba) {
        let i = (y as usize * self.width as usize + x as usize) * 4;
        self.data[i] = color.r;
        self.data[i + 1] = color.g;
        self.data[i + 2] = color.b;
        self.data[i + 3] = color.a;
    }

    /// RGB after compositing over opaque white.
    #[inline]
    pub fn rgb_over_white(&self, x: u32, y: u32) -> [f32; 3] {
        let p = self.pixel(x, y);
        let a = p[3] as f32 / 255.0;
        [
            (p[0] as f32 * a + 255.0 * (1.0 - a)) / 255.0,
            (p[1] as f32 * a + 255.0 * (1.0 - a)) / 255.0,
            (p[2] as f32 * a + 255.0 * (1.0 - a)) / 255.0,
        ]
    }

    /// Perceptual luminance in `0..=1` (Rec. 709 weights), composited over white.
    #[inline]
    pub fn luminance(&self, x: u32, y: u32) -> f32 {
        let [r, g, b] = self.rgb_over_white(x, y);
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_pixels_read_as_white() {
        let r = Raster::new(1, 1);
        assert!((r.luminance(0, 0) - 1.0).abs() < 1e-6);
        assert_eq!(r.rgb_over_white(0, 0), [1.0, 1.0, 1.0]);
    }

    #[test]
    fn black_pixels_read_as_zero() {
        let mut r = Raster::new(1, 1);
        r.set_pixel(0, 0, Rgba::BLACK);
        assert!(r.luminance(0, 0).abs() < 1e-6);
    }
}
