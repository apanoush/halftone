//! Loading and saving `config.toml`.
//!
//! Configuration is layered. From lowest to highest precedence:
//!
//! 1. Built-in presets compiled into the crate.
//! 2. The user config, from the platform config directory
//!    (`~/.config/halftone/config.toml` on Linux).
//! 3. The project config, `./config.toml`, when present.
//!
//! An explicit path replaces layers 2 and 3: the result is the built-ins plus
//! that single file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use toml::Value;

use crate::error::{Error, Result};
use crate::preset::AppConfig;
use crate::settings::{HalftoneSettings, SettingsPatch};

const APP_QUALIFIER: &str = "";
const APP_ORGANIZATION: &str = "";
const APP_NAME: &str = "halftone";
const CONFIG_FILE: &str = "config.toml";

/// On-disk configuration shape.
#[derive(Debug, Default, Serialize, Deserialize)]
struct ConfigFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_preset: Option<String>,
    #[serde(default)]
    presets: BTreeMap<String, SettingsPatch>,
}

/// Path to the user config file, when the platform provides one.
///
/// On Linux this is `$XDG_CONFIG_HOME/halftone/config.toml`, normally
/// `~/.config/halftone/config.toml`.
pub fn user_config_path() -> Option<PathBuf> {
    ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME)
        .map(|dirs| dirs.config_dir().join(CONFIG_FILE))
}

/// Path to `./config.toml`, when a project config exists.
pub fn project_config_path() -> Option<PathBuf> {
    let candidate = PathBuf::from(CONFIG_FILE);
    candidate.exists().then_some(candidate)
}

/// Load the layered configuration.
///
/// With an explicit path, only the built-ins and that file are used and a
/// missing file is an error. Without one, the user config and then the project
/// config are applied over the built-ins.
pub fn load(explicit: Option<&Path>) -> Result<AppConfig> {
    if let Some(path) = explicit {
        if !path.exists() {
            return Err(Error::Config(format!(
                "config file not found: {}",
                path.display()
            )));
        }
        return apply_file(AppConfig::default(), path);
    }
    load_from_paths(
        user_config_path().as_deref(),
        project_config_path().as_deref(),
    )
}

/// Load a specific set of layers, skipping any that do not exist.
///
/// Exposed for tests and for callers that resolve paths themselves.
pub fn load_from_paths(user: Option<&Path>, project: Option<&Path>) -> Result<AppConfig> {
    let mut config = AppConfig::default();
    for path in [user, project].into_iter().flatten() {
        if path.exists() {
            config = apply_file(config, path)?;
        }
    }
    Ok(config)
}

/// Load a single file, merged over the built-in presets.
pub fn load_file(path: &Path) -> Result<AppConfig> {
    apply_file(AppConfig::default(), path)
}

/// Configuration to use as the base when saving.
///
/// Layered when no explicit path is given. An explicit path that does not yet
/// exist yields the built-ins, so a new file can be created.
pub fn load_for_save(explicit: Option<&Path>) -> Result<AppConfig> {
    match explicit {
        Some(path) if !path.exists() => Ok(AppConfig::default()),
        _ => load(explicit),
    }
}

/// Where presets are written when no explicit path is given.
///
/// Defaults to the user config, falling back to `./config.toml` when the
/// platform provides no config directory.
pub fn save_target(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path.to_path_buf());
    }
    Ok(user_config_path().unwrap_or_else(|| PathBuf::from(CONFIG_FILE)))
}

fn apply_file(mut config: AppConfig, path: &Path) -> Result<AppConfig> {
    let file = read_file(path)?;
    config = config.with_overrides(file.default_preset, file.presets);
    Ok(config)
}

fn read_file(path: &Path) -> Result<ConfigFile> {
    let text = std::fs::read_to_string(path)?;
    toml::from_str(&text).map_err(|e| Error::Config(format!("{}: {e}", path.display())))
}

/// Write the complete configuration, including built-in presets.
pub fn write(path: &Path, config: &AppConfig) -> Result<()> {
    let text = to_toml(config)?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

/// Persist one preset, creating the file when needed.
///
/// Existing presets in the target file are preserved.
pub fn save_preset(path: &Path, name: &str, settings: &HalftoneSettings) -> Result<()> {
    let mut config = if path.exists() {
        load_file(path)?
    } else {
        AppConfig::default()
    };
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

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("halftone-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir.join(name)
    }

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

    #[test]
    fn project_layer_overrides_user_preset() {
        let user = temp_file("user.toml");
        std::fs::write(
            &user,
            "default_preset = \"personal\"\n\n[presets.wide]\ncell_size = 20.0\n",
        )
        .unwrap();
        let project = temp_file("project.toml");
        std::fs::write(&project, "[presets.wide]\ncell_size = 3.0\n").unwrap();

        let cfg = load_from_paths(Some(&user), Some(&project)).unwrap();
        // The project file does not set a default, so the user default survives.
        assert_eq!(cfg.default_preset, "personal");
        assert_eq!(cfg.resolve("wide").unwrap().cell_size, 3.0);
    }

    #[test]
    fn missing_layers_are_skipped() {
        let user = temp_file("absent-user.toml");
        let project = temp_file("absent-project.toml");
        let cfg = load_from_paths(Some(&user), Some(&project)).unwrap();
        assert_eq!(cfg.default_preset, AppConfig::default().default_preset);
        assert!(cfg.resolve("classic").is_some());
    }

    #[test]
    fn explicit_path_replaces_other_layers() {
        let explicit = temp_file("explicit.toml");
        std::fs::write(
            &explicit,
            "default_preset = \"solo\"\n\n[presets.solo]\nangle = 12.0\n",
        )
        .unwrap();
        let cfg = load(Some(&explicit)).unwrap();
        assert_eq!(cfg.default_preset, "solo");
        assert_eq!(cfg.resolve("solo").unwrap().angle, 12.0);
        // Built-ins remain available alongside the explicit file.
        assert!(cfg.resolve("classic").is_some());
    }

    #[test]
    fn missing_explicit_path_is_an_error() {
        let missing = temp_file("does-not-exist.toml");
        assert!(load(Some(&missing)).is_err());
    }

    #[test]
    fn malformed_file_reports_the_path() {
        let broken = temp_file("broken.toml");
        std::fs::write(&broken, "this is not = = toml").unwrap();
        let error = load(Some(&broken)).unwrap_err().to_string();
        assert!(error.contains("broken.toml"), "error was: {error}");
    }

    #[test]
    fn save_target_prefers_the_explicit_path() {
        let explicit = temp_file("target.toml");
        assert_eq!(save_target(Some(&explicit)).unwrap(), explicit);
    }

    #[test]
    fn user_config_path_ends_in_halftone_config_toml() {
        let Some(path) = user_config_path() else {
            return;
        };
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("config.toml")
        );
        assert!(
            path.ancestors()
                .any(|part| part.file_name().and_then(|name| name.to_str()) == Some("halftone")),
            "no halftone directory in {}",
            path.display()
        );
    }
}
