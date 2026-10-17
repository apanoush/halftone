//! Rasterize the generated SVG for on-screen preview.
//!
//! The preview is produced from the exact SVG that gets exported, so the GUI
//! is what-you-see-is-what-you-get.

use iced::widget::image::Handle;

/// A ready-to-display preview image.
pub struct PreviewImage {
    pub handle: Handle,
}

/// Rasterize an SVG, scaled so its longest side is at most `max` pixels.
pub fn rasterize_svg(svg: &str, max: u32) -> Option<PreviewImage> {
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(svg, &options).ok()?;
    let size = tree.size();
    let (source_width, source_height) = (size.width(), size.height());
    if source_width <= 0.0 || source_height <= 0.0 {
        return None;
    }
    let scale = (max as f32 / source_width).min(max as f32 / source_height);
    let width = (source_width * scale).round().max(1.0) as u32;
    let height = (source_height * scale).round().max(1.0) as u32;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)?;
    // Fill white so the preview reads clearly and the premultiplied buffer
    // equals straight RGBA.
    pixmap.fill(resvg::tiny_skia::Color::WHITE);
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Some(PreviewImage {
        handle: Handle::from_rgba(width, height, pixmap.data().to_vec()),
    })
}
