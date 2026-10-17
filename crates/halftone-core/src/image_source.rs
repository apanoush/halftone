//! Image decoding, resizing, and output-size resolution.

use std::path::Path;

use image::imageops::FilterType;

use crate::error::Result;
use crate::raster::Raster;

/// Load any format supported by the `image` crate into an RGBA8 raster.
pub fn load(path: &Path) -> Result<Raster> {
    let reader = image::ImageReader::open(path)?.with_guessed_format()?;
    let image = reader.decode()?.to_rgba8();
    Ok(Raster::from_rgba8(
        image.width(),
        image.height(),
        image.into_raw(),
    ))
}

/// Resize with Lanczos filtering. Returns the input unchanged when the
/// dimensions already match.
pub fn resize(raster: &Raster, width: u32, height: u32) -> Raster {
    if raster.width == width && raster.height == height {
        return raster.clone();
    }
    let Some(buffer) = image::RgbaImage::from_raw(raster.width, raster.height, raster.data.clone())
    else {
        return raster.clone();
    };
    let resized = image::imageops::resize(&buffer, width, height, FilterType::Lanczos3);
    Raster::from_rgba8(width, height, resized.into_raw())
}

/// Resolve requested dimensions against the source size, preserving aspect
/// ratio when only one axis is given.
pub fn resolve_dimensions(
    source_width: u32,
    source_height: u32,
    width: Option<u32>,
    height: Option<u32>,
) -> (u32, u32) {
    match (width, height) {
        (Some(w), Some(h)) => (w.max(1), h.max(1)),
        (Some(w), None) => {
            let w = w.max(1);
            let h = ((source_height as f32) * (w as f32 / source_width.max(1) as f32))
                .round()
                .max(1.0) as u32;
            (w, h)
        }
        (None, Some(h)) => {
            let h = h.max(1);
            let w = ((source_width as f32) * (h as f32 / source_height.max(1) as f32))
                .round()
                .max(1.0) as u32;
            (w, h)
        }
        (None, None) => (source_width.max(1), source_height.max(1)),
    }
}

/// Load an image and resize it to the requested canvas.
pub fn load_for_canvas(
    path: &Path,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<(Raster, (u32, u32))> {
    let source = load(path)?;
    let dims = resolve_dimensions(source.width, source.height, width, height);
    Ok((resize(&source, dims.0, dims.1), dims))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_aspect_when_one_axis_given() {
        assert_eq!(resolve_dimensions(400, 200, Some(100), None), (100, 50));
        assert_eq!(resolve_dimensions(400, 200, None, Some(100)), (200, 100));
        assert_eq!(resolve_dimensions(400, 200, None, None), (400, 200));
        assert_eq!(resolve_dimensions(400, 200, Some(50), Some(70)), (50, 70));
    }
}
