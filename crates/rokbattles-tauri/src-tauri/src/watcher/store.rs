use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs, io,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use tauri::{AppHandle, Manager};

use super::config::WatcherConfig;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct ProcessedStore {
    pub(crate) entries: HashMap<String, FileSig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct UploadQueueStore {
    pub(crate) version: u32,
    pub(crate) items: Vec<QueuedUpload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QueuedUpload {
    pub(crate) path: String,
    pub(crate) sig: FileSig,
    pub(crate) attempts: u32,
    pub(crate) not_before_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct FileSig {
    pub(crate) size: u64,
    pub(crate) modified: u128,
}

pub(crate) fn delete_processed(app: &AppHandle, config: &WatcherConfig) -> anyhow::Result<()> {
    let path = processed_file(app, config)?;
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("Failed removing {:?}", path))?;
    }

    Ok(())
}

pub(crate) fn delete_upload_queue(app: &AppHandle, config: &WatcherConfig) -> anyhow::Result<()> {
    let path = upload_queue_file(app, config)?;
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("Failed removing {:?}", path))?;
    }
    Ok(())
}

pub(crate) fn file_sig(meta: &fs::Metadata) -> anyhow::Result<FileSig> {
    let size = meta.len();
    let modified_time = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let modified = modified_time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis();
    Ok(FileSig { size, modified })
}

pub(crate) fn read_processed(
    app: &AppHandle,
    config: &WatcherConfig,
) -> anyhow::Result<ProcessedStore> {
    let path = processed_file(app, config)?;
    if !path.exists() {
        return Ok(ProcessedStore::default());
    }
    let data = fs::read(&path).with_context(|| format!("Failed reading {:?}", path))?;
    if data.is_empty() {
        return Ok(ProcessedStore::default());
    }
    let store: ProcessedStore =
        serde_json::from_slice(&data).with_context(|| format!("Invalid JSON in {:?}", path))?;
    Ok(store)
}

pub(crate) fn write_processed(
    app: &AppHandle,
    config: &WatcherConfig,
    store: &ProcessedStore,
) -> anyhow::Result<()> {
    let path = processed_file(app, config)?;
    let json = serde_json::to_vec(store).context("Failed to serialize processed store to JSON")?;
    atomic_write(&path, &json).with_context(|| format!("Failed writing {:?}", path))?;
    Ok(())
}

pub(crate) fn read_upload_queue(
    app: &AppHandle,
    config: &WatcherConfig,
) -> anyhow::Result<UploadQueueStore> {
    let path = upload_queue_file(app, config)?;
    if !path.exists() {
        return Ok(UploadQueueStore::default());
    }
    let data = fs::read(&path).with_context(|| format!("Failed reading {:?}", path))?;
    if data.is_empty() {
        return Ok(UploadQueueStore::default());
    }
    let store: UploadQueueStore =
        serde_json::from_slice(&data).with_context(|| format!("Invalid JSON in {:?}", path))?;
    Ok(store)
}

pub(crate) fn write_upload_queue(
    app: &AppHandle,
    config: &WatcherConfig,
    store: &UploadQueueStore,
) -> anyhow::Result<()> {
    let path = upload_queue_file(app, config)?;
    let json = serde_json::to_vec(store).context("Failed to serialize upload queue to JSON")?;
    atomic_write(&path, &json).with_context(|| format!("Failed writing {:?}", path))?;
    Ok(())
}

fn processed_file(app: &AppHandle, config: &WatcherConfig) -> anyhow::Result<PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .context("Could not resolve app config directory")?;
    fs::create_dir_all(&dir).context("Failed to create app config directory")?;
    Ok(dir.join(config.processed_file_name))
}

fn upload_queue_file(app: &AppHandle, config: &WatcherConfig) -> anyhow::Result<PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .context("Could not resolve app config directory")?;
    fs::create_dir_all(&dir).context("Failed to create app config directory")?;
    Ok(dir.join(config.upload_queue_file_name))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let tmp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp_path)
        .with_context(|| format!("Failed creating temp file {:?}", tmp_path))?;
    file.write_all(bytes)
        .with_context(|| format!("Failed writing temp file {:?}", tmp_path))?;

    if let Err(e) = fs::rename(&tmp_path, path) {
        // Best-effort fallback for Windows rename semantics.
        if e.kind() == io::ErrorKind::AlreadyExists {
            let _ = fs::remove_file(path);
            fs::rename(&tmp_path, path).with_context(|| format!("Failed replacing {:?}", path))?;
        } else {
            return Err(e).with_context(|| format!("Failed renaming {:?} -> {:?}", tmp_path, path));
        }
    }
    Ok(())
}
