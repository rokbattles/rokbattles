use crate::{read_api_ingress_url, read_dirs};
use anyhow::Context;
use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs, io,
    io::Read,
    io::Write,
    path::Path,
    path::PathBuf,
    sync::OnceLock,
    time::{Duration, Instant, SystemTime},
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{mpsc, watch};

// TODO Replace JSON store with SQLite (or sled)
//  Migration: import existing processed.json on first run, then remove it.
const PROCESSED_FILE: &str = "processed-v2.json";
const LEGACY_PROCESSED_FILE: &str = "processed.json";
const UPLOAD_QUEUE_FILE: &str = "upload-queue.json";
const STORE_FLUSH_EVERY_UPDATES: usize = 20_000;
const STORE_FLUSH_INTERVAL: Duration = Duration::from_secs(300);
const DIR_REFRESH_INTERVAL_IDLE: Duration = Duration::from_secs(5);
const DIR_REFRESH_INTERVAL_BUSY: Duration = Duration::from_secs(60);
const FULL_DIR_REFRESH_INTERVAL: Duration = Duration::from_secs(180);
const SCAN_BUDGET_PER_TICK: usize = 256;
const IDLE_SLEEP: Duration = Duration::from_millis(750);
const QUEUE_FLUSH_INTERVAL: Duration = Duration::from_secs(2);
const QUEUE_FLUSH_EVERY_UPDATES: usize = 64;
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);
const UPLOAD_PREFETCH_TARGET: usize = 4;
const METRICS_INTERVAL: Duration = Duration::from_secs(30);
const HOT_TRACKED_LIMIT: usize = 4096;
const HOT_RESCAN_INTERVAL: Duration = Duration::from_millis(750);
const HOT_RESCAN_BUDGET: usize = 64;
const FILE_STABLE_AGE_MS: u128 = 1500;
const FILE_RETRY_DELAY_MS: u128 = 750;
const FILE_CHANGED_DELAY_MS: u128 = 1000;
const FULL_REFRESH_VALIDATE_RECENT: usize = 5000;
const FULL_REFRESH_VALIDATE_MAX_PATHS: usize = 512;
const FS_EVENT_BUDGET: usize = 512;
const FS_EVENT_QUEUE_CAPACITY: usize = 4096;
const CONFIG_REFRESH_INTERVAL: Duration = Duration::from_secs(2);

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProcessedStore {
    entries: HashMap<String, FileSig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct UploadQueueStore {
    version: u32,
    items: Vec<QueuedUpload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueuedUpload {
    path: String,
    sig: FileSig,
    attempts: u32,
    not_before_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
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

fn upload_queue_file(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .context("Could not resolve app config directory")?;
    fs::create_dir_all(&dir).context("Failed to create app config directory")?;
    Ok(dir.join(UPLOAD_QUEUE_FILE))
}

pub fn delete_processed(app: &AppHandle) -> anyhow::Result<()> {
    let path = processed_file(app)?;
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("Failed removing {:?}", path))?;
    }

    let dir = path.parent().map(Path::to_path_buf);
    if let Some(dir) = dir {
        let legacy = dir.join(LEGACY_PROCESSED_FILE);
        if legacy.exists() {
            fs::remove_file(&legacy).with_context(|| format!("Failed removing {:?}", legacy))?;
        }
    }
    Ok(())
}

pub fn delete_upload_queue(app: &AppHandle) -> anyhow::Result<()> {
    let path = upload_queue_file(app)?;
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

fn write_processed(app: &AppHandle, store: &ProcessedStore) -> anyhow::Result<()> {
    let path = processed_file(app)?;
    let json = serde_json::to_vec(store).context("Failed to serialize processed store to JSON")?;
    atomic_write(&path, &json).with_context(|| format!("Failed writing {:?}", path))?;
    Ok(())
}

fn read_upload_queue(app: &AppHandle) -> anyhow::Result<UploadQueueStore> {
    let path = upload_queue_file(app)?;
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

fn write_upload_queue(app: &AppHandle, store: &UploadQueueStore) -> anyhow::Result<()> {
    let path = upload_queue_file(app)?;
    let json = serde_json::to_vec(store).context("Failed to serialize upload queue to JSON")?;
    atomic_write(&path, &json).with_context(|| format!("Failed writing {:?}", path))?;
    Ok(())
}

fn parse_rok_mail_id(filename: &str) -> Option<u128> {
    let rest = filename.strip_prefix("Persistent.Mail.")?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse::<u128>().ok()
}

fn detect_mail_type<'a>(mail: &'a mail_decoder::MailBorrowed<'a>) -> Option<&'a str> {
    for section in &mail.sections {
        let mail_decoder::ValueBorrowed::Object(entries) = section else {
            continue;
        };
        for (key, value) in entries {
            if *key != "type" {
                continue;
            }
            let mail_decoder::ValueBorrowed::String(value) = value else {
                return None;
            };
            return Some(value);
        }
    }
    None
}

fn has_rok_fileheader_from_file(path: &PathBuf) -> anyhow::Result<bool> {
    let mut f = fs::File::open(path)
        .with_context(|| format!("Failed to open file for header check: {:?}", path))?;
    let mut buf = [0u8; 32];
    let _ = f.read(&mut buf)?;
    Ok(mail_decoder::has_rok_mail_header(&buf))
}

#[derive(Debug, Clone)]
struct DirScan {
    dir: PathBuf,
    ids: Vec<u128>,
    known_ids: HashSet<u128>,
    cursor: usize,
    last_refresh: Instant,
    max_id: Option<u128>,
    last_full_refresh: Instant,
}

impl DirScan {
    fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            ids: Vec::new(),
            known_ids: HashSet::new(),
            cursor: 0,
            last_refresh: Instant::now()
                .checked_sub(DIR_REFRESH_INTERVAL_BUSY)
                .unwrap_or_else(Instant::now),
            max_id: None,
            last_full_refresh: Instant::now()
                .checked_sub(FULL_DIR_REFRESH_INTERVAL)
                .unwrap_or_else(Instant::now),
        }
    }

    fn is_stale(&self) -> bool {
        let interval = if self.cursor == 0 {
            DIR_REFRESH_INTERVAL_IDLE
        } else {
            DIR_REFRESH_INTERVAL_BUSY
        };
        self.ids.is_empty() || self.last_refresh.elapsed() >= interval
    }

    fn peek_next_id(&self) -> Option<u128> {
        if self.cursor == 0 {
            None
        } else {
            Some(self.ids[self.cursor - 1])
        }
    }

    fn pop_next_id(&mut self) -> Option<u128> {
        if self.cursor == 0 {
            return None;
        }
        self.cursor -= 1;
        Some(self.ids[self.cursor])
    }

    fn path_for_id(&self, id: u128) -> PathBuf {
        self.dir.join(format!("Persistent.Mail.{id}"))
    }
}

struct WatcherState {
    scans: Vec<DirScan>,
    dirs: Vec<PathBuf>,
    dirs_last_read: Instant,
    store: ProcessedStore,
    store_dirty_updates: usize,
    store_last_flush: Instant,
    upload_queue: VecDeque<QueuedUpload>,
    upload_queued_paths: HashSet<String>,
    upload_queue_dirty_updates: usize,
    upload_queue_last_flush: Instant,
    scan_refreshes: u64,
    store_flushes: u64,
    upload_queue_flushes: u64,
    hot_paths: VecDeque<String>,
    hot_set: HashSet<String>,
    hot_last_scan: Instant,
}

impl WatcherState {
    fn new(store: ProcessedStore, upload_queue_store: UploadQueueStore) -> Self {
        let mut upload_queue = VecDeque::new();
        let mut upload_queued_paths = HashSet::new();
        for item in upload_queue_store.items {
            if upload_queued_paths.insert(item.path.clone()) {
                upload_queue.push_back(item);
            }
        }

        Self {
            scans: Vec::new(),
            dirs: Vec::new(),
            dirs_last_read: Instant::now()
                .checked_sub(CONFIG_REFRESH_INTERVAL)
                .unwrap_or_else(Instant::now),
            store,
            store_dirty_updates: 0,
            store_last_flush: Instant::now(),
            upload_queue,
            upload_queued_paths,
            upload_queue_dirty_updates: 0,
            upload_queue_last_flush: Instant::now(),
            scan_refreshes: 0,
            store_flushes: 0,
            upload_queue_flushes: 0,
            hot_paths: VecDeque::new(),
            hot_set: HashSet::new(),
            hot_last_scan: Instant::now(),
        }
    }

    fn maybe_flush_store(&mut self, app: &AppHandle) {
        if self.store_dirty_updates == 0 {
            return;
        }
        if self.store_dirty_updates < STORE_FLUSH_EVERY_UPDATES
            && self.store_last_flush.elapsed() < STORE_FLUSH_INTERVAL
        {
            return;
        }
        if let Err(e) = write_processed(app, &self.store) {
            eprintln!("[rokbattles] failed to flush processed store: {}", e);
            return;
        }
        self.store_dirty_updates = 0;
        self.store_last_flush = Instant::now();
        self.store_flushes += 1;
    }

    fn maybe_flush_upload_queue(&mut self, app: &AppHandle) {
        if self.upload_queue_dirty_updates == 0 {
            return;
        }
        if self.upload_queue_dirty_updates < QUEUE_FLUSH_EVERY_UPDATES
            && self.upload_queue_last_flush.elapsed() < QUEUE_FLUSH_INTERVAL
        {
            return;
        }
        let store = UploadQueueStore {
            version: 1,
            items: self.upload_queue.iter().cloned().collect(),
        };
        if let Err(e) = write_upload_queue(app, &store) {
            eprintln!("[rokbattles] failed to flush upload queue: {}", e);
            return;
        }
        self.upload_queue_dirty_updates = 0;
        self.upload_queue_last_flush = Instant::now();
        self.upload_queue_flushes += 1;
    }

    fn track_hot_path(&mut self, path: String) {
        if self.hot_set.contains(&path) {
            return;
        }
        if self.hot_paths.len() >= HOT_TRACKED_LIMIT
            && let Some(old) = self.hot_paths.pop_front()
        {
            self.hot_set.remove(&old);
        }
        self.hot_set.insert(path.clone());
        self.hot_paths.push_back(path);
    }

    fn enqueue_upload(&mut self, item: QueuedUpload) {
        let path = item.path.clone();
        if !self.upload_queued_paths.insert(item.path.clone()) {
            return;
        }
        self.upload_queue.push_back(item);
        self.upload_queue_dirty_updates += 1;
        self.track_hot_path(path);
    }

    fn pop_ready_upload(&mut self, now_ms: u128) -> Option<QueuedUpload> {
        let len = self.upload_queue.len();
        for _ in 0..len {
            let item = self.upload_queue.pop_front()?;
            if item.not_before_ms.is_some_and(|t| t > now_ms) {
                self.upload_queue.push_back(item);
                continue;
            }
            self.upload_queued_paths.remove(&item.path);
            self.upload_queue_dirty_updates += 1;
            return Some(item);
        }
        None
    }

    fn requeue_upload(&mut self, item: QueuedUpload) {
        if self.upload_queued_paths.insert(item.path.clone()) {
            self.upload_queue.push_back(item);
            self.upload_queue_dirty_updates += 1;
        }
    }

    fn maybe_rescan_hot(&mut self, now_ms: u128) {
        if self.hot_last_scan.elapsed() < HOT_RESCAN_INTERVAL {
            return;
        }
        self.hot_last_scan = Instant::now();

        for _ in 0..HOT_RESCAN_BUDGET {
            let Some(path_str) = self.hot_paths.pop_front() else {
                break;
            };

            let path = PathBuf::from(&path_str);
            let Some(existing) = self.store.entries.get(&path_str).cloned() else {
                self.hot_paths.push_back(path_str);
                continue;
            };

            let meta = match fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => {
                    self.hot_paths.push_back(path_str);
                    continue;
                }
            };
            let sig = match file_sig(&meta) {
                Ok(s) => s,
                Err(_) => {
                    self.hot_paths.push_back(path_str);
                    continue;
                }
            };

            if sig != existing {
                self.store.entries.insert(path_str.clone(), sig.clone());
                self.store_dirty_updates += 1;
                self.enqueue_upload(QueuedUpload {
                    path: path_str.clone(),
                    sig,
                    attempts: 0,
                    not_before_ms: Some(now_ms.saturating_add(FILE_CHANGED_DELAY_MS)),
                });
            }

            self.hot_paths.push_back(path_str);
        }
    }
}

async fn refresh_scans_if_needed(app: &AppHandle, state: &mut WatcherState) -> bool {
    let next_dirs =
        if !state.dirs.is_empty() && state.dirs_last_read.elapsed() < CONFIG_REFRESH_INTERVAL {
            state.dirs.clone()
        } else {
            let dirs = match read_dirs(app) {
                Ok(d) => d,
                Err(e) => {
                    emit_log(app, format!("Failed to read config: {}", e));
                    eprintln!("[rokbattles] failed to read config: {}", e);
                    return false;
                }
            };
            let next_dirs: Vec<PathBuf> = dirs.into_iter().map(PathBuf::from).collect();
            state.dirs = next_dirs.clone();
            state.dirs_last_read = Instant::now();
            next_dirs
        };
    let needs_rebuild = state.scans.len() != next_dirs.len()
        || state
            .scans
            .iter()
            .zip(next_dirs.iter())
            .any(|(scan, dir)| &scan.dir != dir);

    if needs_rebuild {
        state.scans = next_dirs.into_iter().map(DirScan::new).collect();
    }

    let mut did_refresh = false;
    let mut refresh_count = 0u64;
    let mut full_refreshed_dirs: Vec<PathBuf> = Vec::new();
    for scan in &mut state.scans {
        if !scan.is_stale() {
            continue;
        }
        #[derive(Debug)]
        enum RefreshResult {
            Full(Vec<u128>),
            New(Vec<u128>),
        }

        let do_full = scan.last_full_refresh.elapsed() >= FULL_DIR_REFRESH_INTERVAL;
        let dir = scan.dir.clone();
        let max_id = scan.max_id;
        let ids = tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<RefreshResult> {
            let read_dir = fs::read_dir(&dir)
                .with_context(|| format!("Failed to read directory {:?}", dir))?;

            let mut ids = Vec::new();
            for entry in read_dir.flatten() {
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if !file_type.is_file() {
                    continue;
                }
                let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                    continue;
                };
                let Some(id) = parse_rok_mail_id(&name) else {
                    continue;
                };

                if !do_full
                    && let Some(max) = max_id
                    && id <= max
                {
                    continue;
                }
                ids.push(id);
            }

            ids.sort_unstable();
            ids.dedup();

            if do_full || max_id.is_none() {
                Ok(RefreshResult::Full(ids))
            } else {
                Ok(RefreshResult::New(ids))
            }
        })
        .await;

        match ids {
            Ok(Ok(RefreshResult::Full(ids))) => {
                if scan.ids.is_empty() {
                    scan.max_id = ids.last().copied();
                    scan.known_ids = ids.iter().copied().collect();
                    scan.ids = ids;
                    scan.cursor = scan.ids.len();
                } else {
                    for id in ids {
                        if scan.known_ids.insert(id) {
                            scan.ids.insert(scan.cursor, id);
                            scan.cursor = scan.cursor.saturating_add(1);
                            scan.max_id = Some(scan.max_id.map_or(id, |m| m.max(id)));
                        }
                    }
                }

                scan.last_refresh = Instant::now();
                scan.last_full_refresh = Instant::now();
                did_refresh = true;
                refresh_count += 1;
                full_refreshed_dirs.push(scan.dir.clone());
            }
            Ok(Ok(RefreshResult::New(ids))) => {
                for id in ids {
                    if scan.known_ids.insert(id) {
                        scan.ids.insert(scan.cursor, id);
                        scan.cursor = scan.cursor.saturating_add(1);
                        scan.max_id = Some(scan.max_id.map_or(id, |m| m.max(id)));
                    }
                }
                scan.last_refresh = Instant::now();
                did_refresh = true;
                refresh_count += 1;
            }
            Ok(Err(e)) => {
                eprintln!(
                    "[rokbattles] directory scan failed for {:?}: {}",
                    scan.dir, e
                );
            }
            Err(e) => {
                eprintln!(
                    "[rokbattles] directory scan task failed for {:?}: {}",
                    scan.dir, e
                );
            }
        }
    }

    state.scan_refreshes = state.scan_refreshes.saturating_add(refresh_count);

    if !full_refreshed_dirs.is_empty() {
        let now_ms = now_epoch_ms();
        for dir in full_refreshed_dirs {
            let Some(scan) = state.scans.iter().find(|s| s.dir == dir) else {
                continue;
            };

            let recent_ids = scan
                .ids
                .iter()
                .rev()
                .take(FULL_REFRESH_VALIDATE_RECENT)
                .copied()
                .collect::<Vec<u128>>();

            let mut validate_paths: Vec<PathBuf> = Vec::new();
            let mut validate_keys: Vec<String> = Vec::new();
            for id in recent_ids {
                let path = scan.path_for_id(id);
                let key = path.to_string_lossy().to_string();
                if state.store.entries.contains_key(&key) {
                    validate_paths.push(path);
                    validate_keys.push(key);
                    if validate_paths.len() >= FULL_REFRESH_VALIDATE_MAX_PATHS {
                        break;
                    }
                }
            }

            if validate_paths.is_empty() {
                continue;
            }

            let results = tauri::async_runtime::spawn_blocking(move || {
                validate_paths
                    .into_iter()
                    .zip(validate_keys)
                    .filter_map(|(path, key)| {
                        let meta = fs::metadata(&path).ok()?;
                        let sig = file_sig(&meta).ok()?;
                        Some((key, sig))
                    })
                    .collect::<Vec<(String, FileSig)>>()
            })
            .await;

            let Ok(results) = results else {
                continue;
            };

            for (key, sig_now) in results {
                let Some(existing) = state.store.entries.get(&key).cloned() else {
                    continue;
                };
                if existing == sig_now {
                    continue;
                }
                state.store.entries.insert(key.clone(), sig_now.clone());
                state.store_dirty_updates += 1;
                state.enqueue_upload(QueuedUpload {
                    path: key,
                    sig: sig_now,
                    attempts: 0,
                    not_before_ms: Some(now_ms.saturating_add(FILE_CHANGED_DELAY_MS)),
                });
            }
        }
    }

    did_refresh
}

async fn next_file(app: &AppHandle, state: &mut WatcherState) -> Option<QueuedUpload> {
    for _ in 0..SCAN_BUDGET_PER_TICK {
        let (scan_idx, _best_id) = state
            .scans
            .iter()
            .enumerate()
            .filter_map(|(idx, scan)| scan.peek_next_id().map(|id| (idx, id)))
            .max_by_key(|(_, id)| *id)?;

        let scan = &mut state.scans[scan_idx];
        let id = scan.pop_next_id()?;
        let path = scan.path_for_id(id);
        let key = path.to_string_lossy().to_string();

        if state.store.entries.contains_key(&key) {
            continue;
        }

        let meta = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let sig = match file_sig(&meta) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // check file header (blocking IO)
        let header = tauri::async_runtime::spawn_blocking({
            let path = path.clone();
            move || has_rok_fileheader_from_file(&path)
        })
        .await;

        match header {
            Ok(Ok(true)) => {
                state.store.entries.insert(key, sig.clone());
                state.store_dirty_updates += 1;
                state.maybe_flush_store(app);
                return Some(QueuedUpload {
                    path: path.to_string_lossy().to_string(),
                    sig,
                    attempts: 0,
                    not_before_ms: None,
                });
            }
            Ok(Ok(false)) => {
                state.store.entries.insert(key, sig);
                state.store_dirty_updates += 1;
                state.maybe_flush_store(app);
            }
            Ok(Err(e)) => {
                eprintln!("[rokbattles] header check failed on {:?}: {}", path, e)
            }
            Err(e) => {
                eprintln!("[rokbattles] header check task failed on {:?}: {}", path, e)
            }
        }
    }

    state.maybe_flush_store(app);
    None
}

fn now_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis()
}

fn upload_backoff(attempts: u32) -> Duration {
    let seconds = 2u64.saturating_pow(attempts.min(10));
    Duration::from_secs(seconds.clamp(2, 300))
}

fn sync_fs_watches(
    watcher: Option<&mut RecommendedWatcher>,
    watched_dirs: &mut HashSet<PathBuf>,
    desired_dirs: &[PathBuf],
) {
    let Some(watcher) = watcher else {
        return;
    };

    let desired: HashSet<PathBuf> = desired_dirs.iter().cloned().collect();

    for dir in watched_dirs
        .difference(&desired)
        .cloned()
        .collect::<Vec<_>>()
    {
        if let Err(e) = watcher.unwatch(&dir) {
            eprintln!("[rokbattles] failed to unwatch {:?}: {}", dir, e);
        }
        watched_dirs.remove(&dir);
    }

    for dir in desired
        .difference(watched_dirs)
        .cloned()
        .collect::<Vec<_>>()
    {
        if let Err(e) = watcher.watch(&dir, RecursiveMode::NonRecursive) {
            eprintln!("[rokbattles] failed to watch {:?}: {}", dir, e);
            continue;
        }
        watched_dirs.insert(dir);
    }
}

fn apply_fs_event(state: &mut WatcherState, path: PathBuf, now_ms: u128) {
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return;
    };
    let Some(id) = parse_rok_mail_id(name) else {
        return;
    };

    let meta = match fs::metadata(&path) {
        Ok(m) => m,
        Err(_) => return,
    };
    let sig = match file_sig(&meta) {
        Ok(s) => s,
        Err(_) => return,
    };

    let key = path.to_string_lossy().to_string();
    if let Some(existing) = state.store.entries.get(&key)
        && *existing == sig
    {
        return;
    }
    state.store.entries.insert(key.clone(), sig.clone());
    state.store_dirty_updates += 1;
    state.enqueue_upload(QueuedUpload {
        path: key,
        sig,
        attempts: 0,
        not_before_ms: Some(now_ms.saturating_add(FILE_RETRY_DELAY_MS)),
    });

    if let Some(parent) = path.parent()
        && let Some(scan) = state.scans.iter_mut().find(|s| s.dir == parent)
    {
        let _ = scan.known_ids.insert(id);
        scan.max_id = Some(scan.max_id.map_or(id, |m| m.max(id)));
    }
}

#[derive(Debug)]
struct UploadApiError {
    status: Option<reqwest::StatusCode>,
    retry_after: Option<Duration>,
    message: String,
}

pub struct WatcherTask {
    shutdown: watch::Sender<bool>,
    handle: tauri::async_runtime::JoinHandle<()>,
}

impl WatcherTask {
    pub async fn shutdown(self, app: &AppHandle) {
        let _ = self.shutdown.send(true);
        let mut handle = self.handle;
        tokio::select! {
            _ = &mut handle => {}
            _ = tokio::time::sleep(SHUTDOWN_TIMEOUT) => {
                emit_log(app, "Watcher shutdown timed out; aborting task");
                handle.abort();
            }
        }
    }
}

async fn post_file_to_api(
    client: &reqwest::Client,
    api_url: &str,
    bytes: Vec<u8>,
) -> Result<(), UploadApiError> {
    let resp = client
        .post(api_url)
        .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
        .body(bytes)
        .send()
        .await
        .map_err(|e| UploadApiError {
            status: None,
            retry_after: None,
            message: format!("failed to send mail to API: {e}"),
        })?;

    let status = resp.status();
    if !status.is_success() {
        let retry_after = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs);
        let text = resp.text().await.unwrap_or_default();
        return Err(UploadApiError {
            status: Some(status),
            retry_after,
            message: format!("API rejected upload: {} {}", status, text),
        });
    }
    Ok(())
}

pub fn spawn_watcher(app: &AppHandle) -> WatcherTask {
    let app = app.clone();

    let api_url = read_api_ingress_url(&app)
        .unwrap_or_else(|_| "https://rokbattles.com/api/v1/ingress".to_string());
    let client = http_client();

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let (fs_tx, mut fs_rx) = mpsc::channel::<PathBuf>(FS_EVENT_QUEUE_CAPACITY);
    let handle = tauri::async_runtime::spawn(async move {
        let processed = match read_processed(&app) {
            Ok(store) => store,
            Err(e) => {
                emit_log(&app, format!("Failed to read processed store: {}", e));
                eprintln!("[rokbattles] failed to read processed store: {}", e);
                ProcessedStore::default()
            }
        };
        let upload_queue = match read_upload_queue(&app) {
            Ok(store) => store,
            Err(e) => {
                emit_log(&app, format!("Failed to read upload queue: {}", e));
                eprintln!("[rokbattles] failed to read upload queue: {}", e);
                UploadQueueStore::default()
            }
        };
        let mut state = WatcherState::new(processed, upload_queue);
        let mut fs_watcher = match notify::recommended_watcher({
            let fs_tx = fs_tx.clone();
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    for path in event.paths {
                        let _ = fs_tx.try_send(path);
                    }
                }
            }
        }) {
            Ok(w) => Some(w),
            Err(e) => {
                eprintln!(
                    "[rokbattles] failed to initialize filesystem watcher: {}",
                    e
                );
                None
            }
        };
        let mut fs_watched_dirs: HashSet<PathBuf> = HashSet::new();

        let mut last_metrics = Instant::now();
        let mut metrics_processed: u64 = 0;
        let mut metrics_uploaded: u64 = 0;
        let mut metrics_skipped: u64 = 0;
        let mut metrics_decode_failed: u64 = 0;
        let mut metrics_upload_failed: u64 = 0;
        let mut metrics_fs_events: u64 = 0;

        loop {
            if *shutdown_rx.borrow() {
                state.maybe_flush_store(&app);
                state.maybe_flush_upload_queue(&app);
                break;
            }

            let now_ms = now_epoch_ms();

            state.maybe_flush_store(&app);
            state.maybe_flush_upload_queue(&app);
            state.maybe_rescan_hot(now_ms);

            let _ = refresh_scans_if_needed(&app, &mut state).await;
            sync_fs_watches(fs_watcher.as_mut(), &mut fs_watched_dirs, &state.dirs);

            for _ in 0..FS_EVENT_BUDGET {
                let Ok(path) = fs_rx.try_recv() else {
                    break;
                };
                metrics_fs_events += 1;
                apply_fs_event(&mut state, path, now_ms);
            }

            while state.upload_queue.len() < UPLOAD_PREFETCH_TARGET {
                if let Some(item) = next_file(&app, &mut state).await {
                    state.enqueue_upload(item);
                } else {
                    break;
                }
            }

            if last_metrics.elapsed() >= METRICS_INTERVAL {
                eprintln!(
                    "[rokbattles] metrics: processed={} uploaded={} skipped={} decode_failed={} upload_failed={} fs_events={} queue={} hot={} processed_entries={} dir_refreshes={} store_flushes={} queue_flushes={}",
                    metrics_processed,
                    metrics_uploaded,
                    metrics_skipped,
                    metrics_decode_failed,
                    metrics_upload_failed,
                    metrics_fs_events,
                    state.upload_queue.len(),
                    state.hot_paths.len(),
                    state.store.entries.len(),
                    state.scan_refreshes,
                    state.store_flushes,
                    state.upload_queue_flushes,
                );
                last_metrics = Instant::now();
            }

            if let Some(item) = state.pop_ready_upload(now_ms) {
                let path = PathBuf::from(&item.path);
                let fname = filename_only(&path);
                metrics_processed += 1;
                emit_log(&app, format!("Processing {}", fname));
                eprintln!("[rokbattles] processing {:?}", path);

                let meta = match fs::metadata(&path) {
                    Ok(m) => m,
                    Err(e) => {
                        emit_log(&app, format!("Failed to stat file {}: {}", fname, e));
                        eprintln!("[rokbattles] failed to stat file {:?}: {}", path, e);
                        continue;
                    }
                };
                let sig_now = match file_sig(&meta) {
                    Ok(s) => s,
                    Err(e) => {
                        emit_log(&app, format!("Failed to stat file {}: {}", fname, e));
                        eprintln!("[rokbattles] failed to stat file {:?}: {}", path, e);
                        continue;
                    }
                };
                let age_ms = now_ms.saturating_sub(sig_now.modified);
                if sig_now != item.sig || age_ms < FILE_STABLE_AGE_MS {
                    let path_key = item.path.clone();
                    let mut next = item;
                    next.sig = sig_now.clone();
                    next.not_before_ms = Some(now_ms.saturating_add(FILE_RETRY_DELAY_MS));
                    state.requeue_upload(next);

                    state.store.entries.insert(path_key, sig_now);
                    state.store_dirty_updates += 1;
                    continue;
                }

                let bytes = match tauri::async_runtime::spawn_blocking({
                    let path = path.clone();
                    move || fs::read(&path)
                })
                .await
                {
                    Ok(Ok(b)) => b,
                    Ok(Err(e)) => {
                        emit_log(&app, format!("Failed to read file {}: {}", fname, e));
                        eprintln!("[rokbattles] failed to read file {:?}: {}", path, e);

                        let mut next = item;
                        next.attempts = next.attempts.saturating_add(1);
                        let backoff = upload_backoff(next.attempts);
                        next.not_before_ms = Some(now_ms.saturating_add(backoff.as_millis()));
                        state.requeue_upload(next);
                        continue;
                    }
                    Err(e) => {
                        emit_log(&app, format!("Failed to read file {}: {}", fname, e));
                        eprintln!("[rokbattles] read task failed for {:?}: {}", path, e);

                        let mut next = item;
                        next.attempts = next.attempts.saturating_add(1);
                        let backoff = upload_backoff(next.attempts);
                        next.not_before_ms = Some(now_ms.saturating_add(backoff.as_millis()));
                        state.requeue_upload(next);
                        continue;
                    }
                };

                let decoded = match mail_decoder::decode(&bytes) {
                    Ok(m) => m,
                    Err(e) => {
                        metrics_decode_failed += 1;
                        emit_log(&app, format!("Decode failed for {}: {}", fname, e));
                        eprintln!("[rokbattles] decode failed for {:?}: {}", path, e);
                        continue;
                    }
                };

                let first_type = detect_mail_type(&decoded);
                if !first_type.is_some_and(|t| t.eq_ignore_ascii_case("Battle")) {
                    metrics_skipped += 1;
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

                match post_file_to_api(client, &api_url, bytes).await {
                    Ok(()) => {
                        metrics_uploaded += 1;
                        emit_log(&app, format!("Uploaded {}", fname));
                        eprintln!("[rokbattles] uploaded battle mail {:?}", path);
                    }
                    Err(e) => {
                        metrics_upload_failed += 1;
                        emit_log(&app, format!("Failed to upload {}: {}", fname, e.message));
                        eprintln!("[rokbattles] failed to upload {:?}: {}", path, e.message);

                        let retryable = e.status.is_none_or(|s| s == 429 || s.is_server_error());
                        if retryable {
                            let mut next = item;
                            next.attempts = next.attempts.saturating_add(1);
                            let backoff = e
                                .retry_after
                                .unwrap_or_else(|| upload_backoff(next.attempts));
                            next.not_before_ms = Some(now_ms.saturating_add(backoff.as_millis()));
                            state.requeue_upload(next);
                        }
                    }
                }
                continue;
            }

            let now_ms = now_epoch_ms();
            tokio::select! {
                _ = tokio::time::sleep(IDLE_SLEEP) => {}
                res = shutdown_rx.changed() => {
                    if res.is_ok() && *shutdown_rx.borrow() {
                        state.maybe_flush_store(&app);
                        state.maybe_flush_upload_queue(&app);
                        break;
                    }
                }
                maybe_path = fs_rx.recv() => {
                    if let Some(path) = maybe_path {
                        metrics_fs_events += 1;
                        apply_fs_event(&mut state, path, now_ms);
                    }
                }
            }
        }
    });

    WatcherTask {
        shutdown: shutdown_tx,
        handle,
    }
}
