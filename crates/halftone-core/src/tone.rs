//! Tone curve applied before halftoning.

use crate::settings::HalftoneSettings;

/// Brightness / contrast / gamma / invert, applied on normalized values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tone {
    pub brightness: f32,
    pub contrast: f32,
    pub gamma: f32,
    pub invert: bool,
}

impl Tone {
    pub fn from_settings(settings: &HalftoneSettings) -> Self {
        Self {
            brightness: settings.brightness,
            contrast: settings.contrast,
            gamma: settings.gamma,
            invert: settings.invert,
        }
    }

    pub fn identity() -> Self {
        Self {
            brightness: 0.0,
            contrast: 1.0,
            gamma: 1.0,
            invert: false,
        }
    }

    /// Map a normalized input value. The result stays in `0..=1`.
    pub fn adjust(&self, value: f32) -> f32 {
        let mut v = value.clamp(0.0, 1.0);
        if self.invert {
            v = 1.0 - v;
        }
        v = (v + self.brightness).clamp(0.0, 1.0);
        v = ((v - 0.5) * self.contrast + 0.5).clamp(0.0, 1.0);
        // Display gamma: values above 1 lighten the source.
        v = v.powf(1.0 / self.gamma.max(1e-4));
        v.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_identity() {
        let tone = Tone::identity();
        for v in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert!((tone.adjust(v) - v).abs() < 1e-6);
        }
    }

    #[test]
    fn invert_flips() {
        let tone = Tone {
            invert: true,
            ..Tone::identity()
        };
        assert!((tone.adjust(0.0) - 1.0).abs() < 1e-6);
        assert!(tone.adjust(1.0).abs() < 1e-6);
    }

    #[test]
    fn gamma_above_one_lightens() {
        let tone = Tone {
            gamma: 2.0,
            ..Tone::identity()
        };
        assert!(tone.adjust(0.5) > 0.5);
    }
}
