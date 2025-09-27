use crate::{read_api_ingress_url, read_dirs};
use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::Path,
    path::PathBuf,
    sync::OnceLock,
    time::{Duration, SystemTime},
};
use tauri::{AppHandle, Emitter, Manager};

// TODO Replace JSON store with SQLite (or sled)
//  Migration: import existing processed.json on first run, then remove it.
const PROCESSED_FILE: &str = "processed.json";
// TODO Fetch rate limit from API (or response headers) on startup and adjust dynamically.
//  If app remains open for long sessions, monitor headers to keep this value in sync.
const RATE_LIMIT: u32 = 128;
const TICK: u64 = 60000 / RATE_LIMIT as u64;

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProcessedStore {
    entries: HashMap<String, FileSig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileSig {
    size: u64,
    modified: u128,
}

#[derive(Debug, Clone, Serialize)]
struct LogPayload {
    message: String,
}

fn build_user_agent() -> String {
    // Format: ROKBattles/<app_version> (<os>; <arch>) Tauri/<tauri_version>
    format!(
        "ROKBattles/{version} ({os}; {arch}) Tauri/{tauri}",
        version = env!("CARGO_PKG_VERSION"),
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        tauri = tauri::VERSION
    )
}

fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        let user_agent = build_user_agent();
        reqwest::Client::builder()
            .user_agent(user_agent)
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .pool_idle_timeout(Some(Duration::from_secs(90)))
            .pool_max_idle_per_host(8)
            // TODO Add connect/read timeouts and a retry policy with exponential backoff
            .build()
            .expect("failed to build HTTP client")
    })
}

fn emit_log(app: &AppHandle, message: impl Into<String>) {
    let _ = app.emit(
        "rokbattles",
        LogPayload {
            message: message.into(),
        },
    );
}

fn filename_only(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string()
}

fn processed_file(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .context("Could not resolve app config directory")?;
    fs::create_dir_all(&dir).context("Failed to create app config directory")?;
    Ok(dir.join(PROCESSED_FILE))
}

pub fn delete_processed(app: &AppHandle) -> anyhow::Result<()> {
    let path = processed_file(app)?;
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("Failed removing {:?}", path))?;
    }
    Ok(())
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
    // TODO Atomic write (write to temp + fsync + rename)
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

fn has_rok_fileheader_from_file(path: &PathBuf) -> anyhow::Result<bool> {
    let mut f = fs::File::open(path)
        .with_context(|| format!("Failed to open file for header check: {:?}", path))?;
    let mut buf = [0u8; 32];
    let _ = f.read(&mut buf)?;
    Ok(mail_decoder::has_rok_mail_header(&buf))
}

async fn next_file(app: &AppHandle) -> Option<PathBuf> {
    let dirs = match read_dirs(app) {
        Ok(d) => d,
        Err(e) => {
            emit_log(app, format!("Failed to read config: {}", e));
            eprintln!("[rokbattles] failed to read config: {}", e);
            return None;
        }
    };

    let mut store = match read_processed(app) {
        Ok(s) => s,
        Err(e) => {
            emit_log(app, format!("Failed to read processed store: {}", e));
            eprintln!("[rokbattles] failed to read processed store: {}", e);
            return None;
        }
    };

    // TODO Replace periodic scans with OS file notifications (notify crate) and maintain a pending queue.
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
                    // TODO Debounce/flush writes every N updates or on a timer.
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

async fn post_file_to_api(
    client: &reqwest::Client,
    api_url: &str,
    bytes: &[u8],
) -> anyhow::Result<()> {
    let resp = client
        .post(api_url)
        .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
        .body(bytes.to_vec())
        .send()
        .await
        .context("failed to send mail to API")?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        // TODO Apply backoff+retry for 429/5xx. Persist failed attempts for later retry.
        bail!("API rejected upload: {} {}", status, text);
    }
    Ok(())
}

pub fn spawn_watcher(app: &AppHandle) {
    let app = app.clone();

    let api_url = read_api_ingress_url(&app)
        .unwrap_or_else(|_| "https://rokbattles.com/api/v1/ingress".to_string());
    let client = http_client();

    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(TICK));
        loop {
            ticker.tick().await;

            match next_file(&app).await {
                Some(path) => {
                    let fname = filename_only(&path);
                    emit_log(&app, format!("Processing {}", fname));
                    eprintln!("[rokbattles] processing {:?}", path);

                    // TODO Use tokio::fs or spawn_blocking for file I/O to avoid blocking the async runtime.
                    let bytes = match fs::read(&path) {
                        Ok(b) => b,
                        Err(e) => {
                            emit_log(&app, format!("Failed to read file {}: {}", fname, e));
                            eprintln!("[rokbattles] failed to read file {:?}: {}", path, e);
                            continue;
                        }
                    };

                    let decoded = match mail_decoder::decode(&bytes) {
                        Ok(m) => m,
                        Err(e) => {
                            emit_log(&app, format!("Decode failed for {}: {}", fname, e));
                            eprintln!("[rokbattles] decode failed for {:?}: {}", path, e);
                            continue;
                        }
                    };

                    let first_type = mail_helper::detect_mail_type_str(&decoded);
                    if !first_type.is_some_and(|t| t.eq_ignore_ascii_case("Battle")) {
                        emit_log(
                            &app,
                            format!(
                                "Skipping non-battle mail {} (detected: {})",
                                fname,
                                first_type.unwrap_or("Unknown")
                            ),
                        );
                        eprintln!(
                            "[rokbattles] skipping non-battle mail {:?} (detected: {:?})",
                            path,
                            first_type.unwrap_or("Unknown")
                        );
                        continue;
                    }

                    // TODO When offline/API unavailable, queue uploads locally and retry.
                    if let Err(e) = post_file_to_api(client, &api_url, &bytes).await {
                        emit_log(&app, format!("Failed to upload {}: {}", fname, e));
                        eprintln!("[rokbattles] failed to upload {:?}: {}", path, e);
                    } else {
                        emit_log(&app, format!("Uploaded {}", fname));
                        eprintln!("[rokbattles] uploaded battle mail {:?}", path);
                    }
                }
                None => {
                    tokio::time::sleep(Duration::from_millis(TICK)).await;
                }
            }
        }
    });
}
