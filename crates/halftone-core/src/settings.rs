//! Conversion settings and their serializable patch form.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::color::Rgba;
use crate::error::{Error, Result};

/// Halftone pattern family.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HalftoneStyle {
    /// Classic amplitude-modulated dot grid.
    #[default]
    Dots,
    /// Parallel variable-width lines.
    Lines,
    /// Layered lines at three angles.
    Crosshatch,
    /// Clustered ordered (Bayer) dither.
    Ordered,
    /// Floyd-Steinberg error diffusion on a cell grid.
    Diffusion,
}

impl FromStr for HalftoneStyle {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "dots" | "dot" => Ok(Self::Dots),
            "lines" | "line" | "hatch" => Ok(Self::Lines),
            "crosshatch" | "cross" => Ok(Self::Crosshatch),
            "ordered" | "bayer" => Ok(Self::Ordered),
            "diffusion" | "floyd-steinberg" => Ok(Self::Diffusion),
            other => Err(format!("unknown style '{other}'")),
        }
    }
}

impl fmt::Display for HalftoneStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Dots => "dots",
            Self::Lines => "lines",
            Self::Crosshatch => "crosshatch",
            Self::Ordered => "ordered",
            Self::Diffusion => "diffusion",
        };
        f.write_str(name)
    }
}

/// Shape of an AM dot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DotShape {
    #[default]
    Circle,
    Square,
    Diamond,
    Ellipse,
    Cross,
}

impl FromStr for DotShape {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "circle" => Ok(Self::Circle),
            "square" => Ok(Self::Square),
            "diamond" => Ok(Self::Diamond),
            "ellipse" => Ok(Self::Ellipse),
            "cross" => Ok(Self::Cross),
            other => Err(format!("unknown dot shape '{other}'")),
        }
    }
}

impl fmt::Display for DotShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Circle => "circle",
            Self::Square => "square",
            Self::Diamond => "diamond",
            Self::Ellipse => "ellipse",
            Self::Cross => "cross",
        };
        f.write_str(name)
    }
}

/// Output color handling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColorMode {
    /// Single ink layer.
    #[default]
    Mono,
    /// Four ink layers at classic screen angles.
    Cmyk,
}

impl FromStr for ColorMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "mono" | "monochrome" => Ok(Self::Mono),
            "cmyk" => Ok(Self::Cmyk),
            other => Err(format!("unknown color mode '{other}'")),
        }
    }
}

impl fmt::Display for ColorMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Mono => "mono",
            Self::Cmyk => "cmyk",
        })
    }
}

/// Complete conversion configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HalftoneSettings {
    pub style: HalftoneStyle,
    /// Halftone cell size in output pixels.
    pub cell_size: f32,
    pub dot_shape: DotShape,
    /// Screen angle in degrees.
    pub angle: f32,
    pub invert: bool,
    /// Contrast multiplier around mid-gray. `1.0` is neutral.
    pub contrast: f32,
    /// Brightness offset in `-1..=1`. `0.0` is neutral.
    pub brightness: f32,
    /// Display gamma. `1.0` is neutral, values above 1 lighten.
    pub gamma: f32,
    pub color_mode: ColorMode,
    pub foreground: Rgba,
    /// SVG background. Transparent means no background rectangle.
    pub background: Rgba,
    /// Output width in pixels. `None` uses the source size.
    pub width: Option<u32>,
    /// Output height in pixels. `None` uses the source size.
    pub height: Option<u32>,
    /// Scales dot coverage. `1.0` reaches full density at full black.
    pub max_dot_scale: f32,
    /// Ordered dither matrix size, 4 or 8.
    pub bayer_size: u8,
}

impl Default for HalftoneSettings {
    fn default() -> Self {
        Self {
            style: HalftoneStyle::Dots,
            cell_size: 6.0,
            dot_shape: DotShape::Circle,
            angle: 45.0,
            invert: false,
            contrast: 1.0,
            brightness: 0.0,
            gamma: 1.0,
            color_mode: ColorMode::Mono,
            foreground: Rgba::BLACK,
            background: Rgba::TRANSPARENT,
            width: None,
            height: None,
            max_dot_scale: 1.0,
            bayer_size: 8,
        }
    }
}

impl HalftoneSettings {
    /// Check ranges that the algorithms rely on.
    pub fn validate(&self) -> Result<()> {
        let fail = |msg: String| Err(Error::Settings(msg));
        if !self.cell_size.is_finite() || self.cell_size < 0.5 {
            return fail(format!(
                "cell_size must be at least 0.5 px, got {}",
                self.cell_size
            ));
        }
        if !self.contrast.is_finite() || self.contrast <= 0.0 {
            return fail("contrast must be greater than 0".into());
        }
        if !self.gamma.is_finite() || self.gamma <= 0.0 {
            return fail("gamma must be greater than 0".into());
        }
        if !self.max_dot_scale.is_finite() || !(0.05..=2.0).contains(&self.max_dot_scale) {
            return fail("max_dot_scale must be between 0.05 and 2.0".into());
        }
        if self.bayer_size != 4 && self.bayer_size != 8 {
            return fail("bayer_size must be 4 or 8".into());
        }
        for (label, dim) in [("width", self.width), ("height", self.height)] {
            if let Some(v) = dim {
                if v == 0 || v > 16_384 {
                    return fail(format!("{label} must be between 1 and 16384 pixels"));
                }
            }
        }
        Ok(())
    }

    /// A patch containing every field, used when persisting a preset.
    pub fn to_patch(&self) -> SettingsPatch {
        SettingsPatch {
            style: Some(self.style),
            cell_size: Some(self.cell_size),
            dot_shape: Some(self.dot_shape),
            angle: Some(self.angle),
            invert: Some(self.invert),
            contrast: Some(self.contrast),
            brightness: Some(self.brightness),
            gamma: Some(self.gamma),
            color_mode: Some(self.color_mode),
            foreground: Some(self.foreground),
            background: Some(self.background),
            width: Some(self.width),
            height: Some(self.height),
            max_dot_scale: Some(self.max_dot_scale),
            bayer_size: Some(self.bayer_size),
        }
    }
}

/// A partial set of settings, applied on top of a base configuration.
///
/// Presets use this type so the TOML file only contains the fields that
/// differ from the defaults.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SettingsPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<HalftoneStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cell_size: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dot_shape: Option<DotShape>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub angle: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invert: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contrast: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brightness: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gamma: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_mode: Option<ColorMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground: Option<Rgba>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Rgba>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_dot_scale: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bayer_size: Option<u8>,
}

impl SettingsPatch {
    /// Apply this patch on top of a base configuration.
    pub fn apply(&self, base: &HalftoneSettings) -> HalftoneSettings {
        let mut out = base.clone();
        if let Some(v) = self.style {
            out.style = v;
        }
        if let Some(v) = self.cell_size {
            out.cell_size = v;
        }
        if let Some(v) = self.dot_shape {
            out.dot_shape = v;
        }
        if let Some(v) = self.angle {
            out.angle = v;
        }
        if let Some(v) = self.invert {
            out.invert = v;
        }
        if let Some(v) = self.contrast {
            out.contrast = v;
        }
        if let Some(v) = self.brightness {
            out.brightness = v;
        }
        if let Some(v) = self.gamma {
            out.gamma = v;
        }
        if let Some(v) = self.color_mode {
            out.color_mode = v;
        }
        if let Some(v) = self.foreground {
            out.foreground = v;
        }
        if let Some(v) = self.background {
            out.background = v;
        }
        if let Some(v) = self.width {
            out.width = v;
        }
        if let Some(v) = self.height {
            out.height = v;
        }
        if let Some(v) = self.max_dot_scale {
            out.max_dot_scale = v;
        }
        if let Some(v) = self.bayer_size {
            out.bayer_size = v;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        HalftoneSettings::default().validate().unwrap();
    }

    #[test]
    fn rejects_bad_cell_size() {
        let settings = HalftoneSettings {
            cell_size: 0.0,
            ..HalftoneSettings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn patch_only_overrides_present_fields() {
        let base = HalftoneSettings::default();
        let patch = SettingsPatch {
            cell_size: Some(3.0),
            ..Default::default()
        };
        let out = patch.apply(&base);
        assert_eq!(out.cell_size, 3.0);
        assert_eq!(out.angle, base.angle);
    }

    #[test]
    fn full_patch_round_trips() {
        let s = HalftoneSettings::default();
        assert_eq!(s.to_patch().apply(&HalftoneSettings::default()), s);
    }
}
