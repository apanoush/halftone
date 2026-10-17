//! SVG document model and writer.
//!
//! The writer is deliberately small and dependency free: halftone output only
//! needs circles, rectangles, ellipses and polygons.

use std::fmt::Write as _;

use crate::color::Rgba;
use crate::shape::{Geometry, Layer};

/// A complete SVG document.
#[derive(Clone, Debug, PartialEq)]
pub struct SvgDoc {
    pub width: u32,
    pub height: u32,
    /// Background rectangle. Skipped when fully transparent.
    pub background: Rgba,
    pub layers: Vec<Layer>,
}

impl SvgDoc {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            background: Rgba::TRANSPARENT,
            layers: Vec::new(),
        }
    }

    pub fn shape_count(&self) -> usize {
        self.layers.iter().map(|l| l.shapes.len()).sum()
    }

    /// Serialize to an SVG string.
    pub fn to_svg(&self) -> String {
        let mut out = String::with_capacity(self.shape_count() * 48 + 512);
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        let _ = writeln!(
            out,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" shape-rendering=\"geometricPrecision\">",
            w = self.width,
            h = self.height,
        );

        if !self.background.is_transparent() {
            let _ = writeln!(
                out,
                "  <rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"{}\"{}/>",
                self.width,
                self.height,
                self.background.to_rgb_hex(),
                opacity_attr(self.background, "fill-opacity"),
            );
        }

        for layer in &self.layers {
            if layer.shapes.is_empty() {
                continue;
            }
            let _ = writeln!(
                out,
                "  <g id=\"{}\" fill=\"{}\"{}>",
                escape_id(&layer.name),
                layer.color.to_rgb_hex(),
                opacity_attr(layer.color, "fill-opacity"),
            );
            for shape in &layer.shapes {
                write_shape(&mut out, shape);
            }
            out.push_str("  </g>\n");
        }

        out.push_str("</svg>\n");
        out
    }
}

fn opacity_attr(color: Rgba, attr: &str) -> String {
    if color.a == 255 {
        String::new()
    } else {
        format!(" {attr}=\"{:.3}\"", color.alpha_f32())
    }
}

fn write_shape(out: &mut String, shape: &Geometry) {
    match shape {
        Geometry::Circle { cx, cy, r } => {
            let _ = writeln!(
                out,
                "    <circle cx=\"{}\" cy=\"{}\" r=\"{}\"/>",
                num(*cx),
                num(*cy),
                num(*r)
            );
        }
        Geometry::Rect { x, y, w, h } => {
            let _ = writeln!(
                out,
                "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
                num(*x),
                num(*y),
                num(*w),
                num(*h)
            );
        }
        Geometry::Ellipse {
            cx,
            cy,
            rx,
            ry,
            rotation_deg,
        } => {
            let _ = writeln!(
                out,
                "    <ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" transform=\"rotate({} {} {})\"/>",
                num(*cx),
                num(*cy),
                num(*rx),
                num(*ry),
                num(*rotation_deg),
                num(*cx),
                num(*cy)
            );
        }
        Geometry::Polygon(points) => {
            out.push_str("    <polygon points=\"");
            for (i, [x, y]) in points.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                let _ = write!(out, "{},{}", num(*x), num(*y));
            }
            out.push_str("\"/>\n");
        }
    }
}

/// Format a coordinate with at most two decimals and no trailing zeros.
pub fn num(value: f32) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let text = format!("{value:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text.is_empty() || text == "-0" {
        "0".to_string()
    } else {
        text.to_string()
    }
}

fn escape_id(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_numbers_compactly() {
        assert_eq!(num(10.0), "10");
        assert_eq!(num(0.5), "0.5");
        assert_eq!(num(-0.001), "0");
        assert_eq!(num(1.234), "1.23");
    }

    #[test]
    fn writes_transparent_background_without_rect() {
        let mut doc = SvgDoc::new(10, 20);
        doc.layers.push(Layer::new(
            "ink",
            Rgba::BLACK,
            vec![Geometry::Circle {
                cx: 5.0,
                cy: 5.0,
                r: 2.0,
            }],
        ));
        let svg = doc.to_svg();
        assert!(svg.contains("viewBox=\"0 0 10 20\""));
        assert!(!svg.contains("<rect x=\"0\" y=\"0\""));
        assert!(svg.contains("<circle cx=\"5\" cy=\"5\" r=\"2\"/>"));
    }

    #[test]
    fn writes_background_when_opaque() {
        let mut doc = SvgDoc::new(4, 4);
        doc.background = Rgba::WHITE;
        assert!(doc.to_svg().contains("fill=\"#ffffff\""));
    }
}
