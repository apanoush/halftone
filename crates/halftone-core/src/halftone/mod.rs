//! Halftone pattern generation.
//!
//! All algorithms consume a [`CoverageMap`] and return vector primitives in
//! canvas coordinates. None of them know about color.

pub mod diffusion;
pub mod dots;
pub mod grid;
pub mod lines;
pub mod ordered;

use crate::coverage::CoverageMap;
use crate::settings::{DotShape, HalftoneStyle};
use crate::shape::Geometry;

/// Parameters shared by the pattern generators.
#[derive(Clone, Copy, Debug)]
pub struct PatternOptions {
    /// Cell size (or line spacing) in pixels.
    pub cell: f32,
    /// Screen angle in degrees.
    pub angle: f32,
    pub shape: DotShape,
    /// Coverage multiplier for dot size.
    pub max_scale: f32,
    /// Bayer matrix size (4 or 8).
    pub bayer_size: u8,
}

/// Generate shapes for one coverage map.
pub fn render(style: HalftoneStyle, map: &CoverageMap, opts: PatternOptions) -> Vec<Geometry> {
    match style {
        HalftoneStyle::Dots => dots::render(map, opts.cell, opts.angle, opts.shape, opts.max_scale),
        HalftoneStyle::Lines => lines::render(map, opts.cell, opts.angle, opts.max_scale),
        HalftoneStyle::Crosshatch => {
            lines::render_crosshatch(map, opts.cell, opts.angle, opts.max_scale)
        }
        HalftoneStyle::Ordered => ordered::render(map, opts.cell, opts.bayer_size),
        HalftoneStyle::Diffusion => diffusion::render(map, opts.cell),
    }
}
