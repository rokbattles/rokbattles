use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::PathBuf};
use tauri::{AppHandle, Manager};

const CONFIG_FILE_NAME: &str = "config.json";
const DEFAULT_INGRESS_URL: &str = "https://rokbattles.com/api/v1/ingress";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_ingress_url", rename = "ingressUrl")]
    pub ingress_url: String,
    #[serde(default, rename = "watchDirs")]
    pub watch_dirs: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ingress_url: DEFAULT_INGRESS_URL.to_string(),
            watch_dirs: Vec::new(),
        }
    }
}

fn default_ingress_url() -> String {
    DEFAULT_INGRESS_URL.to_string()
}

pub fn config_file(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .context("Could not resolve app config directory")?;
    fs::create_dir_all(&dir).context("Failed to create app config directory")?;
    Ok(dir.join(CONFIG_FILE_NAME))
}

pub fn load(app: &AppHandle) -> anyhow::Result<AppConfig> {
    let path = config_file(app)?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let data = fs::read(&path).with_context(|| format!("Failed reading {:?}", path))?;
    if data.is_empty() {
        return Ok(AppConfig::default());
    }

    // Backward compatability
    let v: serde_json::Value =
        serde_json::from_slice(&data).with_context(|| format!("Invalid JSON in {:?}", path))?;
    if v.is_array() {
        let dirs: Vec<String> = serde_json::from_value(v).unwrap_or_default();
        return Ok(AppConfig {
            watch_dirs: dirs,
            ..AppConfig::default()
        });
    }

    if v.is_object() {
        let cfg: Result<AppConfig, _> = serde_json::from_slice(&data);
        if let Ok(cfg) = cfg {
            return Ok(cfg);
        }
        let mut cfg = AppConfig::default();
        if let Some(u) = v.get("ingressUrl").and_then(|x| x.as_str())
            && !u.trim().is_empty()
        {
            cfg.ingress_url = u.to_string();
        }
        if let Some(arr) = v.get("watchDirs").and_then(|x| x.as_array()) {
            cfg.watch_dirs = arr
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect();
        } else if let Some(arr) = v.get("dirs").and_then(|x| x.as_array()) {
            cfg.watch_dirs = arr
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect();
        }
        return Ok(cfg);
    }

    Ok(AppConfig::default())
}

pub fn save(app: &AppHandle, cfg: &AppConfig) -> anyhow::Result<()> {
    let path = config_file(app)?;
    let json = serde_json::to_vec_pretty(cfg).context("Failed to serialize AppConfig to JSON")?;
    fs::write(&path, json).with_context(|| format!("Failed writing {:?}", path))?;
    Ok(())
}

pub fn read_watch_dirs(app: &AppHandle) -> anyhow::Result<Vec<String>> {
    Ok(load(app)?.watch_dirs)
}

pub fn write_watch_dirs(app: &AppHandle, dirs: &[String]) -> anyhow::Result<()> {
    let mut cfg = load(app)?;
    let set: BTreeSet<String> = dirs
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    cfg.watch_dirs = set.into_iter().collect();
    save(app, &cfg)
}

pub fn read_ingress_url(app: &AppHandle) -> anyhow::Result<String> {
    Ok(load(app)?.ingress_url)
}
