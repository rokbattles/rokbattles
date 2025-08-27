use crate::read_dirs;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::PathBuf,
    time::{Duration, SystemTime},
};
use tauri::{AppHandle, Manager};

const PROCESSED_FILE: &str = "processed.json";
// TODO determine what our rate limit will be for the API
const RATE_LIMIT: u32 = 128;
const TICK: u64 = 60000 / RATE_LIMIT as u64;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProcessedStore {
    entries: HashMap<String, FileSig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileSig {
    size: u64,
    modified: u128,
}

fn processed_file(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .context("Could not resolve app config directory")?;
    fs::create_dir_all(&dir).context("Failed to create app config directory")?;
    Ok(dir.join(PROCESSED_FILE))
}

fn file_sig(meta: &fs::Metadata) -> anyhow::Result<FileSig> {
    let size = meta.len();
    let modified_time = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let modified = modified_time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis();
    Ok(FileSig { size, modified })
}

fn read_processed(app: &AppHandle) -> anyhow::Result<ProcessedStore> {
    let path = processed_file(app)?;
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

fn write_processed(app: &AppHandle, store: &ProcessedStore) -> anyhow::Result<()> {
    let path = processed_file(app)?;
    let json =
        serde_json::to_vec_pretty(store).context("Failed to serialize processed store to JSON")?;
    fs::write(&path, json).with_context(|| format!("Failed writing {:?}", path))?;
    Ok(())
}

fn is_rok_filename(filename: &str) -> bool {
    if let Some(rest) = filename.strip_prefix("Persistent.Mail.") {
        !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit())
    } else {
        false
    }
}

// Taken from our mail-decoder test case
fn has_rok_fileheader(buf: &[u8]) -> bool {
    if buf.len() < 32 {
        return false;
    }
    if buf[0] != 0xFF {
        return false;
    }
    if buf[9] != 0x05 || buf[10] != 0x04 {
        return false;
    }
    let len = {
        let start = 11;
        let end = start + 4;
        let Some(bytes) = buf.get(start..end) else {
            return false;
        };
        u32::from_le_bytes(bytes.try_into().unwrap_or([0; 4]))
    };
    if len != 9 {
        return false;
    }
    let start = 15;
    let end = start + 9;
    let Some(bytes) = buf.get(start..end) else {
        return false;
    };
    bytes == b"mailScene"
}

fn has_rok_fileheader_from_file(path: &PathBuf) -> anyhow::Result<bool> {
    let mut f = fs::File::open(path)
        .with_context(|| format!("Failed to open file for header check: {:?}", path))?;
    let mut buf = [0u8; 32];
    let _ = f.read(&mut buf)?;
    Ok(has_rok_fileheader(&buf))
}

async fn next_file(app: &AppHandle) -> Option<PathBuf> {
    let dirs = match read_dirs(app) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[rokbattles] failed to read config: {}", e);
            return None;
        }
    };

    let mut store = match read_processed(app) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[rokbattles] failed to read processed store: {}", e);
            return None;
        }
    };

    for dir in dirs {
        let dir_path = PathBuf::from(dir);
        let read_dir = match fs::read_dir(&dir_path) {
            Ok(rd) => rd,
            Err(e) => {
                eprintln!(
                    "[rokbattles] failed to read directory {:?}: {}",
                    dir_path, e
                );
                continue;
            }
        };

        for entry in read_dir.flatten() {
            let path = entry.path();
            // skip dirs
            if path.is_dir() {
                continue;
            }

            // filter filenames
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if !is_rok_filename(name) {
                continue;
            }

            // signature
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let sig = match file_sig(&meta) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // check processed files
            let key = path.to_string_lossy().to_string();
            if let Some(existing) = store.entries.get(&key)
                && existing.size == sig.size
                && existing.modified == sig.modified
            {
                // unchanged
                continue;
            }

            // check file header
            match has_rok_fileheader_from_file(&path) {
                Ok(true) => {
                    store.entries.insert(key.clone(), sig);
                    if let Err(e) = write_processed(app, &store) {
                        eprintln!("[rokbattles] failed to update processed store: {}", e);
                    }
                    return Some(path);
                }
                Ok(false) => {
                    store.entries.insert(key, sig);
                    if let Err(e) = write_processed(app, &store) {
                        eprintln!("[rokbattles] failed to update processed store: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("[rokbattles] header check failed on {:?}: {}", path, e)
                }
            }
        }
    }

    None
}

pub fn spawn_watcher(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(TICK));
        loop {
            ticker.tick().await;

            match next_file(&app).await {
                Some(path) => {
                    eprintln!("[rokbattles] processing {:?}", path);

                    // TODO post to the API
                }
                None => {
                    tokio::time::sleep(Duration::from_millis(TICK)).await;
                }
            }
        }
    });
}
