use anyhow::{Context, anyhow};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CloseBehavior {
    #[default]
    Ask,
    MinimizeToTray,
    Quit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AppConfig {
    #[serde(default)]
    pub(crate) dirs: Vec<String>,
    #[serde(default)]
    pub(crate) close_behavior: CloseBehavior,
    #[serde(default = "default_auto_update")]
    pub(crate) auto_update: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            dirs: Vec::new(),
            close_behavior: CloseBehavior::Ask,
            auto_update: default_auto_update(),
        }
    }
}

fn default_auto_update() -> bool {
    true
}

fn config_file(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .context("Could not resolve app config directory")?;
    fs::create_dir_all(&dir).context("Failed to create app config directory")?;
    Ok(dir.join("config.json"))
}

fn parse_config_bytes(data: &[u8]) -> anyhow::Result<AppConfig> {
    if data.is_empty() {
        return Ok(AppConfig::default());
    }

    if let Ok(config) = serde_json::from_slice::<AppConfig>(data) {
        return Ok(config);
    }

    // Older builds stored only a list of watched directories.
    if let Ok(legacy_dirs) = serde_json::from_slice::<Vec<String>>(data) {
        return Ok(AppConfig {
            dirs: legacy_dirs,
            ..AppConfig::default()
        });
    }

    Err(anyhow!("Invalid JSON"))
}

pub(crate) fn read_config(app: &AppHandle) -> anyhow::Result<AppConfig> {
    let path = config_file(app)?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let data = fs::read(&path).with_context(|| format!("Failed reading {:?}", path))?;
    parse_config_bytes(&data).with_context(|| format!("Invalid JSON in {:?}", path))
}

pub(crate) fn write_config(app: &AppHandle, config: &AppConfig) -> anyhow::Result<()> {
    let path = config_file(app)?;
    let json = serde_json::to_vec_pretty(config).context("Failed to serialize config to JSON")?;
    fs::write(&path, json).with_context(|| format!("Failed writing {:?}", path))?;
    Ok(())
}

pub(crate) fn read_dirs(app: &AppHandle) -> anyhow::Result<Vec<String>> {
    Ok(read_config(app)?.dirs)
}

pub(crate) fn write_dirs(app: &AppHandle, dirs: &[String]) -> anyhow::Result<()> {
    let mut config = read_config(app)?;
    config.dirs = dirs.to_vec();
    write_config(app, &config)
}

pub(crate) fn get_close_behavior(app: &AppHandle) -> anyhow::Result<CloseBehavior> {
    Ok(read_config(app)?.close_behavior)
}

pub(crate) fn set_close_behavior(app: &AppHandle, behavior: CloseBehavior) -> anyhow::Result<()> {
    let mut config = read_config(app)?;
    config.close_behavior = behavior;
    write_config(app, &config)
}

pub(crate) fn get_auto_update(app: &AppHandle) -> anyhow::Result<bool> {
    Ok(read_config(app)?.auto_update)
}

pub(crate) fn set_auto_update(app: &AppHandle, enabled: bool) -> anyhow::Result<()> {
    let mut config = read_config(app)?;
    config.auto_update = enabled;
    write_config(app, &config)
}

#[cfg(test)]
mod tests {
    use super::{CloseBehavior, parse_config_bytes};

    #[test]
    fn defaults_to_auto_update_true() {
        let config = parse_config_bytes(&[]).expect("default config should parse");
        assert!(config.auto_update);
        assert_eq!(config.close_behavior, CloseBehavior::Ask);
        assert!(config.dirs.is_empty());
    }

    #[test]
    fn reads_new_auto_update_key() {
        let raw = br#"{"dirs":["/tmp/mail"],"close_behavior":"quit","auto_update":false}"#;
        let config = parse_config_bytes(raw).expect("new config should parse");

        assert_eq!(config.dirs, vec!["/tmp/mail"]);
        assert_eq!(config.close_behavior, CloseBehavior::Quit);
        assert!(!config.auto_update);
    }

    #[test]
    fn missing_auto_update_stays_enabled() {
        let raw = br#"{"dirs":["/tmp/mail"],"close_behavior":"ask"}"#;
        let config = parse_config_bytes(raw).expect("config without auto_update should parse");

        assert!(config.auto_update);
    }

    #[test]
    fn reads_legacy_dirs_only_shape() {
        let raw = br#"["/tmp/one","/tmp/two"]"#;
        let config = parse_config_bytes(raw).expect("legacy dirs-only shape should parse");

        assert_eq!(config.dirs, vec!["/tmp/one", "/tmp/two"]);
        assert_eq!(config.close_behavior, CloseBehavior::Ask);
        assert!(config.auto_update);
    }

    #[test]
    fn invalid_json_is_rejected() {
        let err = parse_config_bytes(br#"{"dirs":123}"#).expect_err("invalid config must fail");
        assert!(err.to_string().contains("Invalid JSON"));
    }
}
