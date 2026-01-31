use anyhow::Context;
use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};
use std::{
    collections::HashSet,
    fs,
    io::Read,
    path::PathBuf,
    time::{Duration, Instant},
};
use tauri::AppHandle;

use super::emit_log;
use super::mail::parse_rok_mail_id;
use super::state::WatcherState;
use super::store::{QueuedUpload, file_sig};

#[derive(Debug, Clone)]
pub(crate) struct DirScan {
    pub(crate) dir: PathBuf,
    pub(crate) ids: Vec<u128>,
    pub(crate) known_ids: HashSet<u128>,
    pub(crate) cursor: usize,
    pub(crate) last_refresh: Instant,
    pub(crate) max_id: Option<u128>,
    pub(crate) last_full_refresh: Instant,
}

impl DirScan {
    pub(crate) fn new(
        dir: PathBuf,
        dir_refresh_interval_busy: Duration,
        full_dir_refresh_interval: Duration,
    ) -> Self {
        Self {
            dir,
            ids: Vec::new(),
            known_ids: HashSet::new(),
            cursor: 0,
            last_refresh: Instant::now()
                .checked_sub(dir_refresh_interval_busy)
                .unwrap_or_else(Instant::now),
            max_id: None,
            last_full_refresh: Instant::now()
                .checked_sub(full_dir_refresh_interval)
                .unwrap_or_else(Instant::now),
        }
    }

    fn is_stale(
        &self,
        dir_refresh_interval_idle: Duration,
        dir_refresh_interval_busy: Duration,
    ) -> bool {
        let interval = if self.cursor == 0 {
            dir_refresh_interval_idle
        } else {
            dir_refresh_interval_busy
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

pub(crate) async fn refresh_scans_if_needed(app: &AppHandle, state: &mut WatcherState) -> bool {
    let config_refresh_interval = state.config.config_refresh_interval;
    let dir_refresh_interval_idle = state.config.dir_refresh_interval_idle;
    let dir_refresh_interval_busy = state.config.dir_refresh_interval_busy;
    let full_dir_refresh_interval = state.config.full_dir_refresh_interval;
    let full_refresh_validate_recent = state.config.full_refresh_validate_recent;
    let full_refresh_validate_max_paths = state.config.full_refresh_validate_max_paths;
    let file_changed_delay_ms = state.config.file_changed_delay_ms;
    let next_dirs =
        if !state.dirs.is_empty() && state.dirs_last_read.elapsed() < config_refresh_interval {
            state.dirs.clone()
        } else {
            let dirs = match crate::read_dirs(app) {
                Ok(d) => d,
                Err(e) => {
                    emit_log(app, format!("Failed to read config: {}", e));
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
        state.scans = next_dirs
            .into_iter()
            .map(|dir| DirScan::new(dir, dir_refresh_interval_busy, full_dir_refresh_interval))
            .collect();
    }

    let mut did_refresh = false;
    let mut refresh_count = 0u64;
    let mut full_refreshed_dirs: Vec<PathBuf> = Vec::new();
    for scan in &mut state.scans {
        if !scan.is_stale(dir_refresh_interval_idle, dir_refresh_interval_busy) {
            continue;
        }
        #[derive(Debug)]
        enum RefreshResult {
            Full(Vec<u128>),
            New(Vec<u128>),
        }

        let do_full = scan.last_full_refresh.elapsed() >= full_dir_refresh_interval;
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
                emit_log(
                    app,
                    format!("Directory scan failed for {:?}: {}", scan.dir, e),
                );
            }
            Err(e) => {
                emit_log(
                    app,
                    format!("Directory scan task failed for {:?}: {}", scan.dir, e),
                );
            }
        }
    }

    state.scan_refreshes = state.scan_refreshes.saturating_add(refresh_count);

    if !full_refreshed_dirs.is_empty() {
        let now_ms = super::now_epoch_ms();
        for dir in full_refreshed_dirs {
            let (recent_ids, dir_path) = {
                let Some(scan) = state.scans.iter().find(|s| s.dir == dir) else {
                    continue;
                };
                let recent_ids = scan
                    .ids
                    .iter()
                    .rev()
                    .take(full_refresh_validate_recent)
                    .copied()
                    .collect::<Vec<_>>();
                (recent_ids, scan.dir.clone())
            };
            let mut queued: usize = 0;
            for id in recent_ids {
                if queued >= full_refresh_validate_max_paths {
                    break;
                }
                let path = dir_path.join(format!("Persistent.Mail.{id}"));
                let key = path.to_string_lossy().to_string();
                let Some(existing) = state.store.entries.get(&key).cloned() else {
                    continue;
                };

                let meta = match fs::metadata(&path) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let sig = match file_sig(&meta) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                if sig == existing {
                    continue;
                }
                queued += 1;
                state.store.entries.insert(key.clone(), sig.clone());
                state.store_dirty_updates += 1;
                state.enqueue_upload(QueuedUpload {
                    path: key,
                    sig,
                    attempts: 0,
                    not_before_ms: Some(now_ms.saturating_add(file_changed_delay_ms)),
                });
            }
        }
    }

    did_refresh
}

pub(crate) async fn next_file(app: &AppHandle, state: &mut WatcherState) -> Option<QueuedUpload> {
    for _ in 0..state.config.scan_budget_per_tick {
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
                emit_log(app, format!("Header check failed for {:?}: {}", path, e));
            }
            Err(e) => {
                emit_log(
                    app,
                    format!("Header check task failed for {:?}: {}", path, e),
                );
            }
        }
    }

    state.maybe_flush_store(app);
    None
}

pub(crate) fn sync_fs_watches(
    app: &AppHandle,
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
            emit_log(app, format!("Failed to unwatch {:?}: {}", dir, e));
        }
        watched_dirs.remove(&dir);
    }

    for dir in desired
        .difference(watched_dirs)
        .cloned()
        .collect::<Vec<_>>()
    {
        if let Err(e) = watcher.watch(&dir, RecursiveMode::NonRecursive) {
            emit_log(app, format!("Failed to watch {:?}: {}", dir, e));
            continue;
        }
        watched_dirs.insert(dir);
    }
}

pub(crate) fn apply_fs_event(state: &mut WatcherState, path: PathBuf, now_ms: u128) {
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
        not_before_ms: Some(now_ms.saturating_add(state.config.file_retry_delay_ms)),
    });

    if let Some(parent) = path.parent()
        && let Some(scan) = state.scans.iter_mut().find(|s| s.dir == parent)
    {
        let _ = scan.known_ids.insert(id);
        scan.max_id = Some(scan.max_id.map_or(id, |m| m.max(id)));
    }
}

fn has_rok_fileheader_from_file(path: &PathBuf) -> anyhow::Result<bool> {
    let mut f = fs::File::open(path)
        .with_context(|| format!("Failed to open file for header check: {:?}", path))?;
    let mut buf = [0u8; 32];
    let _ = f.read(&mut buf)?;
    Ok(super::mail::has_rok_mail_header(&buf))
}
