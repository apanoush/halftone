//! Vector primitives produced by the halftone algorithms.

use crate::color::Rgba;

/// A single filled vector primitive in canvas coordinates.
#[derive(Clone, Debug, PartialEq)]
pub enum Geometry {
    Circle {
        cx: f32,
        cy: f32,
        r: f32,
    },
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
    Ellipse {
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        rotation_deg: f32,
    },
    Polygon(Vec<[f32; 2]>),
}

impl Geometry {
    /// Rough bounding box as `(min_x, min_y, max_x, max_y)`.
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        match self {
            Self::Circle { cx, cy, r } => (cx - r, cy - r, cx + r, cy + r),
            Self::Rect { x, y, w, h } => (*x, *y, x + w, y + h),
            Self::Ellipse { cx, cy, rx, ry, .. } => (cx - rx, cy - ry, cx + rx, cy + ry),
            Self::Polygon(points) => {
                let mut min_x = f32::INFINITY;
                let mut min_y = f32::INFINITY;
                let mut max_x = f32::NEG_INFINITY;
                let mut max_y = f32::NEG_INFINITY;
                for [x, y] in points {
                    min_x = min_x.min(*x);
                    min_y = min_y.min(*y);
                    max_x = max_x.max(*x);
                    max_y = max_y.max(*y);
                }
                (min_x, min_y, max_x, max_y)
            }
        }
    }
}

/// A group of shapes sharing one ink color.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub name: String,
    pub color: Rgba,
    pub shapes: Vec<Geometry>,
}

impl Layer {
    pub fn new(name: impl Into<String>, color: Rgba, shapes: Vec<Geometry>) -> Self {
        Self {
            name: name.into(),
            color,
            shapes,
        }
    }
}
