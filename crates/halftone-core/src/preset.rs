//! Named presets and application configuration.

use std::collections::BTreeMap;

use crate::settings::{ColorMode, DotShape, HalftoneSettings, HalftoneStyle, SettingsPatch};

/// Application configuration: a default preset plus a registry of patches.
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub default_preset: String,
    pub presets: BTreeMap<String, SettingsPatch>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_preset: "classic".to_string(),
            presets: builtin_presets(),
        }
    }
}

impl AppConfig {
    /// Resolve a preset name into complete settings.
    pub fn resolve(&self, name: &str) -> Option<HalftoneSettings> {
        self.presets
            .get(name)
            .map(|patch| patch.apply(&HalftoneSettings::default()))
    }

    /// Sorted preset names.
    pub fn preset_names(&self) -> Vec<&str> {
        self.presets.keys().map(String::as_str).collect()
    }

    /// Merge custom presets over the built-ins. Existing names are replaced.
    pub fn with_overrides(
        mut self,
        default_preset: Option<String>,
        presets: BTreeMap<String, SettingsPatch>,
    ) -> Self {
        if let Some(name) = default_preset {
            self.default_preset = name;
        }
        self.presets.extend(presets);
        self
    }
}

fn patch() -> SettingsPatch {
    SettingsPatch::default()
}

/// Presets shipped with the application.
pub fn builtin_presets() -> BTreeMap<String, SettingsPatch> {
    let mut map = BTreeMap::new();

    map.insert(
        "classic".into(),
        SettingsPatch {
            style: Some(HalftoneStyle::Dots),
            cell_size: Some(6.0),
            dot_shape: Some(DotShape::Circle),
            angle: Some(45.0),
            ..patch()
        },
    );

    map.insert(
        "newsprint".into(),
        SettingsPatch {
            style: Some(HalftoneStyle::Dots),
            cell_size: Some(4.0),
            dot_shape: Some(DotShape::Circle),
            angle: Some(45.0),
            contrast: Some(1.15),
            foreground: Some(crate::color::Rgba::rgb(20, 20, 20)),
            ..patch()
        },
    );

    map.insert(
        "poster".into(),
        SettingsPatch {
            style: Some(HalftoneStyle::Dots),
            cell_size: Some(10.0),
            dot_shape: Some(DotShape::Circle),
            angle: Some(15.0),
            max_dot_scale: Some(1.05),
            ..patch()
        },
    );

    map.insert(
        "comic".into(),
        SettingsPatch {
            style: Some(HalftoneStyle::Dots),
            cell_size: Some(6.0),
            dot_shape: Some(DotShape::Circle),
            angle: Some(0.0),
            color_mode: Some(ColorMode::Cmyk),
            ..patch()
        },
    );

    map.insert(
        "ink".into(),
        SettingsPatch {
            style: Some(HalftoneStyle::Lines),
            cell_size: Some(6.0),
            angle: Some(45.0),
            ..patch()
        },
    );

    map.insert(
        "engraving".into(),
        SettingsPatch {
            style: Some(HalftoneStyle::Crosshatch),
            cell_size: Some(5.0),
            angle: Some(45.0),
            ..patch()
        },
    );

    map.insert(
        "bayer".into(),
        SettingsPatch {
            style: Some(HalftoneStyle::Ordered),
            cell_size: Some(4.0),
            bayer_size: Some(8),
            ..patch()
        },
    );

    map.insert(
        "grain".into(),
        SettingsPatch {
            style: Some(HalftoneStyle::Diffusion),
            cell_size: Some(4.0),
            ..patch()
        },
    );

    map.insert(
        "soft".into(),
        SettingsPatch {
            style: Some(HalftoneStyle::Dots),
            cell_size: Some(8.0),
            dot_shape: Some(DotShape::Ellipse),
            angle: Some(30.0),
            brightness: Some(0.05),
            contrast: Some(0.9),
            ..patch()
        },
    );

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_resolves() {
        let cfg = AppConfig::default();
        for name in cfg.preset_names() {
            let settings = cfg.resolve(name).expect("preset resolves");
            settings.validate().unwrap();
        }
    }

    #[test]
    fn unknown_preset_is_none() {
        assert!(AppConfig::default().resolve("does-not-exist").is_none());
    }
}
