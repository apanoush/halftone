//! Loading and saving `config.toml`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use toml::Value;

use crate::error::{Error, Result};
use crate::preset::AppConfig;
use crate::settings::{HalftoneSettings, SettingsPatch};

/// On-disk configuration shape.
#[derive(Debug, Default, Serialize, Deserialize)]
struct ConfigFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_preset: Option<String>,
    #[serde(default)]
    presets: BTreeMap<String, SettingsPatch>,
}

/// Look for `config.toml` next to the working directory when no explicit
/// path is given.
pub fn discover(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return Some(path.to_path_buf());
    }
    let candidate = PathBuf::from("config.toml");
    candidate.exists().then_some(candidate)
}

/// Load a configuration, merged over the built-in presets.
///
/// A missing path yields the built-in configuration.
pub fn load(path: Option<&Path>) -> Result<AppConfig> {
    let Some(path) = path else {
        return Ok(AppConfig::default());
    };
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let text = std::fs::read_to_string(path)?;
    let file: ConfigFile =
        toml::from_str(&text).map_err(|e| Error::Config(format!("{}: {e}", path.display())))?;
    Ok(AppConfig::default().with_overrides(file.default_preset, file.presets))
}

/// Write the complete configuration, including built-in presets.
pub fn write(path: &Path, config: &AppConfig) -> Result<()> {
    let file = ConfigFile {
        default_preset: Some(config.default_preset.clone()),
        presets: config.presets.clone(),
    };
    let text = toml::to_string_pretty(&file)
        .map_err(|e| Error::Config(format!("failed to serialize config: {e}")))?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

/// Persist one preset, creating the file when needed.
pub fn save_preset(path: &Path, name: &str, settings: &HalftoneSettings) -> Result<()> {
    let mut config = load(Some(path))?;
    config.presets.insert(name.to_string(), settings.to_patch());
    write(path, &config)
}

/// Render the configuration as TOML without touching the filesystem.
pub fn to_toml(config: &AppConfig) -> Result<String> {
    let file = ConfigFile {
        default_preset: Some(config.default_preset.clone()),
        presets: config.presets.clone(),
    };
    toml::to_string_pretty(&file)
        .map_err(|e| Error::Config(format!("failed to serialize config: {e}")))
}

/// Parse a TOML value for diagnostics (used by `preset show`).
pub fn describe_patch(patch: &SettingsPatch) -> Result<String> {
    let value = Value::try_from(patch)
        .map_err(|e| Error::Config(format!("failed to inspect preset: {e}")))?;
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_config_round_trips_through_toml() {
        let cfg = AppConfig::default();
        let text = to_toml(&cfg).unwrap();
        let parsed: ConfigFile = toml::from_str(&text).unwrap();
        assert!(parsed.presets.contains_key("classic"));
        assert_eq!(parsed.default_preset.as_deref(), Some("classic"));
    }

    #[test]
    fn custom_preset_overrides_builtin() {
        let text = r#"
            default_preset = "wide"

            [presets.wide]
            cell_size = 20.0
        "#;
        let file: ConfigFile = toml::from_str(text).unwrap();
        let cfg = AppConfig::default().with_overrides(file.default_preset, file.presets);
        let settings = cfg.resolve("wide").unwrap();
        assert_eq!(settings.cell_size, 20.0);
        assert_eq!(settings.style, crate::settings::HalftoneStyle::Dots);
    }
}
