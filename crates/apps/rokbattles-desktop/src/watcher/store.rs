//! Persistent upload snapshots, exclusive writer locking, and legacy migration.
//!
//! A snapshot keeps pending sources, acknowledgements, rejections, and API backoff
//! together so recovery cannot load conflicting versions of those records.
//! [`StateFile`] holds the writer lock across loading, uploading, and the final save.

use std::{
    collections::VecDeque,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use anyhow::{Context, ensure};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tauri::{AppHandle, Manager};

use super::{MAX_REASON_BYTES, mail::mail_id_for_path};

/// Upload records saved together in one versioned snapshot.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UploadStore {
    version: u32,
    /// Exact ingress mail IDs, shared by all local paths with the same ID.
    pub(crate) acknowledged: FxHashSet<String>,
    /// Sources awaiting acknowledgement, including any request in progress.
    pub(crate) pending: VecDeque<QueuedUpload>,
    /// Sources excluded from retries until discovery observes a changed signature.
    pub(crate) rejected: FxHashMap<String, RejectedUpload>,
    /// API-wide retry deadline in milliseconds since the Unix epoch.
    pub(crate) api_backoff_until_ms: Option<u128>,
}

impl Default for UploadStore {
    fn default() -> Self {
        Self {
            version: 1,
            acknowledged: FxHashSet::default(),
            pending: VecDeque::new(),
            rejected: FxHashMap::default(),
            api_backoff_until_ms: None,
        }
    }
}

impl UploadStore {
    fn validate(&self) -> anyhow::Result<()> {
        ensure!(self.version == 1, "Unsupported upload state version {}", self.version);

        for id in &self.acknowledged {
            ensure!(
                !id.is_empty()
                    && id.bytes().all(|byte| byte.is_ascii_digit())
                    && id.parse::<u128>().is_ok(),
                "Invalid acknowledged mail ID {id:?}"
            );
        }

        let mut paths = FxHashSet::default();

        for item in &self.pending {
            ensure!(
                item.last_error.as_ref().is_none_or(|reason| reason.len() <= MAX_REASON_BYTES),
                "Pending failure reason exceeds {MAX_REASON_BYTES} bytes"
            );
            ensure!(paths.insert(&item.path), "Duplicate pending source {:?}", item.path);
            ensure!(!self.rejected.contains_key(&item.path), "Source is both pending and rejected");
            ensure!(
                !mail_id_for_path(&item.path).is_some_and(|id| self.acknowledged.contains(id)),
                "Source is both pending and acknowledged"
            );
        }

        for (path, rejected) in &self.rejected {
            ensure!(
                rejected.reason.len() <= MAX_REASON_BYTES,
                "Rejection reason exceeds {MAX_REASON_BYTES} bytes"
            );
            ensure!(
                !mail_id_for_path(path).is_some_and(|id| self.acknowledged.contains(id)),
                "Source is both rejected and acknowledged"
            );
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct QueuedUpload {
    pub(crate) path: String,
    /// Last observed metadata, or `None` when it could not be read.
    pub(crate) sig: Option<FileSig>,
    pub(crate) attempts: u32,
    /// Earliest attempt time in milliseconds since the Unix epoch, if delayed.
    pub(crate) not_before_ms: Option<u128>,
    pub(crate) last_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RejectedUpload {
    pub(crate) sig: Option<FileSig>,
    pub(crate) reason: String,
}

/// File size and modification time used to detect source changes between attempts.
///
/// Equal signatures can refer to different file contents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileSig {
    pub(crate) size: u64,
    /// Modification time in milliseconds since the Unix epoch.
    pub(crate) modified: u128,
}

/// An upload snapshot path and the exclusive lock guarding its writers.
///
/// Keep this alive until all watcher work and the final save have finished.
/// A stop timeout does not release ownership or allow another watcher to write.
pub(crate) struct StateFile {
    path: PathBuf,
    _lock: fs::File,
}

impl StateFile {
    pub(crate) fn for_app(app: &AppHandle) -> anyhow::Result<Self> {
        let dir = app.path().app_config_dir().context("Could not resolve app config directory")?;

        Self::open(&dir)
    }

    pub(crate) fn open(dir: &Path) -> anyhow::Result<Self> {
        fs::create_dir_all(dir).context("Failed to create app config directory")?;

        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join("upload-state.lock"))
            .context("Failed to open upload state lock")?;
        lock.try_lock().context("Upload state is already in use by another watcher")?;

        Ok(Self { path: dir.join("upload-state-v1.json"), _lock: lock })
    }

    /// Loads and validates the snapshot, migrating legacy files if it is absent.
    ///
    /// Unreadable or invalid state returns an error without replacing the source file.
    pub(crate) fn load(&self) -> anyhow::Result<UploadStore> {
        if let Some(store) = read_json::<UploadStore>(&self.path)? {
            store.validate()?;

            return Ok(store);
        }

        let dir = self.path.parent().context("Upload state has no parent directory")?;
        let store = migrate_legacy(dir)?;

        // Leave the legacy files intact. If this replacement is interrupted, migration repeats.
        self.save(&store)?;

        Ok(store)
    }

    /// Validates and atomically replaces the snapshot with `store`.
    ///
    /// On Unix, a parent-directory sync failure is reported after replacement.
    /// The new snapshot is then visible, but its durability is uncertain.
    pub(crate) fn save(&self, store: &UploadStore) -> anyhow::Result<()> {
        store.validate()?;

        let bytes = serde_json::to_vec(store).context("Failed to serialize upload state")?;

        atomic_write(&self.path, &bytes)
            .with_context(|| format!("Failed writing upload state {:?}", self.path))
    }

    pub(crate) fn reset(&self) -> anyhow::Result<()> {
        // Validate first so reprocessing cannot silently replace a damaged recovery file.
        let _previous = self.load()?;

        self.save(&UploadStore::default())
    }
}

pub(crate) fn file_sig(meta: &fs::Metadata) -> anyhow::Result<FileSig> {
    let modified_time = meta.modified().context("Could not read file modification time")?;
    let modified =
        modified_time.duration_since(SystemTime::UNIX_EPOCH).unwrap_or(Duration::ZERO).as_millis();

    Ok(FileSig { size: meta.len(), modified })
}

fn read_json<T: DeserializeOwned>(path: &Path) -> anyhow::Result<Option<T>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("Failed reading {path:?}")),
    };

    serde_json::from_slice(&bytes)
        .with_context(|| format!("Invalid upload state in {path:?}; original file retained"))
        .map(Some)
}

fn read_legacy<T: DeserializeOwned>(dir: &Path, name: &str) -> anyhow::Result<Option<T>> {
    let path = dir.join(name);

    match read_json(&path)? {
        Some(store) => Ok(Some(store)),
        // The old Windows replacement could remove the destination before renaming its temp.
        None => read_json(&path.with_extension("tmp")),
    }
}

fn migrate_legacy(dir: &Path) -> anyhow::Result<UploadStore> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ProcessedStore {
        entries: FxHashMap<String, FileSig>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct UploadQueueStore {
        version: u32,
        items: Vec<QueuedUpload>,
    }

    let processed = read_legacy::<ProcessedStore>(dir, "processed-v3.json")?;
    let queue = read_legacy::<UploadQueueStore>(dir, "upload-queue.json")?;
    let mut pending = FxHashMap::default();

    if let Some(processed) = processed {
        // Legacy "processed" entries recorded discovery, not an ingress acknowledgement.
        // Retry them so migration cannot mark an unconfirmed upload as complete.
        for (path, sig) in processed.entries {
            pending.insert(
                path.clone(),
                QueuedUpload {
                    path,
                    sig: Some(sig),
                    attempts: 0,
                    not_before_ms: None,
                    last_error: None,
                },
            );
        }
    }

    if let Some(queue) = queue {
        ensure!(queue.version == 1, "Unsupported legacy upload queue version {}", queue.version);

        for item in queue.items {
            pending.insert(item.path.clone(), item);
        }
    }

    Ok(UploadStore { pending: pending.into_values().collect(), ..UploadStore::default() })
}

fn atomic_write(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let dir = path.parent().context("Upload state has no parent directory")?;
    let mut temporary = tempfile::NamedTempFile::new_in(dir).context("Create upload state temp")?;

    temporary.write_all(bytes).context("Write upload state temp")?;
    temporary.as_file().sync_all().context("Sync upload state temp")?;

    // persist replaces the destination on Windows too, without a remove-then-rename gap.
    temporary.persist(path).context("Replace upload state")?;

    #[cfg(unix)]
    fs::File::open(dir)?.sync_all().context("Sync upload state directory")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn migrates_discovery_and_queue_without_inventing_acknowledgements() {
        for (processed, queued) in [(true, false), (false, true), (true, true)] {
            let dir = tempfile::tempdir().expect("state directory");
            let processed_bytes = serde_json::to_vec(&json!({"entries": {
                "Persistent.Mail.1": {"size": 10, "modified": 20},
                "Persistent.Mail.2": {"size": 30, "modified": 40}
            }}))
            .expect("legacy discovery");
            let queued_bytes = serde_json::to_vec(&json!({"version": 1, "items": [{
                "path": "Persistent.Mail.1", "sig": {"size": 50, "modified": 60},
                "attempts": 3, "not_before_ms": 9000
            }]}))
            .expect("legacy queue");

            if processed {
                fs::write(dir.path().join("processed-v3.json"), &processed_bytes)
                    .expect("discovery");
            }

            if queued {
                fs::write(dir.path().join("upload-queue.json"), &queued_bytes).expect("queue");
            }

            let file = StateFile::open(dir.path()).expect("state file");
            let migrated = file.load().expect("migrate legacy state");

            assert!(migrated.acknowledged.is_empty());
            assert_eq!(migrated.pending.len(), if processed { 2 } else { 1 });

            if queued {
                let item = migrated
                    .pending
                    .iter()
                    .find(|item| item.path == "Persistent.Mail.1")
                    .expect("preserved queued item");

                assert_eq!(item.attempts, 3);
                assert_eq!(item.not_before_ms, Some(9000));
                assert_eq!(item.sig, Some(FileSig { size: 50, modified: 60 }));
                assert_eq!(
                    fs::read(dir.path().join("upload-queue.json")).expect("legacy retained"),
                    queued_bytes
                );
            }

            drop(file);

            let file = StateFile::open(dir.path()).expect("restart");
            let restarted = file.load().expect("load migrated snapshot");

            assert_eq!(restarted.pending.len(), migrated.pending.len());
            assert!(restarted.acknowledged.is_empty());
        }
    }

    #[test]
    fn interrupted_migration_repeats_and_recovers_legacy_temporary_files() {
        let dir = tempfile::tempdir().expect("state directory");
        fs::write(
            dir.path().join("processed-v3.tmp"),
            br#"{"entries":{"Persistent.Mail.9":{"size":1,"modified":2}}}"#,
        )
        .expect("old replacement interrupted after removal");
        fs::write(dir.path().join(".tmp-interrupted-upload-state"), b"{partial")
            .expect("new migration interrupted before replacement");

        let file = StateFile::open(dir.path()).expect("state file");
        let migrated = file.load().expect("repeat migration from original data");

        assert_eq!(migrated.pending.front().expect("pending mail").path, "Persistent.Mail.9");
        assert!(migrated.acknowledged.is_empty());
        assert!(dir.path().join("processed-v3.tmp").exists());
    }

    #[test]
    fn corrupt_or_unsupported_state_is_retained_and_blocks_reset() {
        let cases = [
            ("upload-state-v1.json", ""),
            ("upload-state-v1.json", "{partial"),
            (
                "upload-state-v1.json",
                r#"{"version":2,"acknowledged":[],"pending":[],"rejected":{},"api_backoff_until_ms":null}"#,
            ),
            ("processed-v3.json", "not json"),
            ("upload-queue.json", r#"{"version":8,"items":[]}"#),
            ("upload-queue.json", ""),
        ];

        for (name, bytes) in cases {
            let dir = tempfile::tempdir().expect("state directory");
            let path = dir.path().join(name);
            fs::write(&path, bytes).expect("invalid recovery file");
            let file = StateFile::open(dir.path()).expect("state file");

            assert!(file.load().is_err(), "{name}: {bytes}");
            assert!(file.reset().is_err(), "reprocess must not erase corrupt state");
            assert_eq!(fs::read(&path).expect("recovery file retained"), bytes.as_bytes());

            if name != "upload-state-v1.json" {
                assert!(!dir.path().join("upload-state-v1.json").exists());
            }
        }
    }

    #[test]
    fn contradictory_pending_state_is_not_loaded_as_success() {
        let dir = tempfile::tempdir().expect("state directory");
        let bytes = br#"{"version":1,"acknowledged":["123"],"pending":[{"path":"Persistent.Mail.123","sig":null,"attempts":0,"not_before_ms":null,"last_error":null}],"rejected":{},"api_backoff_until_ms":null}"#;
        fs::write(dir.path().join("upload-state-v1.json"), bytes).expect("contradictory state");
        let file = StateFile::open(dir.path()).expect("state file");

        file.load().expect_err("contradictory outcomes must stop the watcher");
        assert_eq!(fs::read(&file.path).expect("original retained"), bytes);
    }

    #[test]
    fn contradictory_rejection_is_retained_and_blocks_loading() {
        let dir = tempfile::tempdir().expect("state directory");
        let bytes = br#"{"version":1,"acknowledged":["123"],"pending":[],"rejected":{"Persistent.Mail.123":{"sig":null,"reason":"rejected"}},"api_backoff_until_ms":null}"#;
        fs::write(dir.path().join("upload-state-v1.json"), bytes).expect("contradictory state");
        let file = StateFile::open(dir.path()).expect("state file");

        file.load().expect_err("acknowledgement and rejection must not coexist");
        file.reset().expect_err("reset must not erase contradictory state");

        assert_eq!(fs::read(&file.path).expect("original retained"), bytes);
    }

    #[test]
    fn replacement_preserves_complete_snapshot_and_ignores_abandoned_temp() {
        let dir = tempfile::tempdir().expect("state directory");
        let file = StateFile::open(dir.path()).expect("state file");
        let mut store = file.load().expect("initial state");
        store.acknowledged.insert("123".to_string());
        file.save(&store).expect("replace existing snapshot");

        fs::write(dir.path().join(".tmp-incomplete"), b"{incomplete")
            .expect("simulate interruption before a later replacement");
        drop(file);

        let file = StateFile::open(dir.path()).expect("restart");
        let restored = file.load().expect("last complete snapshot");

        assert!(restored.acknowledged.contains("123"));
    }

    #[test]
    fn replacement_failure_never_deletes_the_destination() {
        let dir = tempfile::tempdir().expect("state directory");
        let destination = dir.path().join("blocked");
        fs::create_dir(&destination).expect("non-replaceable destination");
        fs::write(destination.join("recovery"), b"original").expect("recovery data");

        assert!(atomic_write(&destination, b"new state").is_err());
        assert_eq!(fs::read(destination.join("recovery")).expect("recovery intact"), b"original");
    }

    #[test]
    fn only_one_watcher_can_write_state_at_a_time() {
        let dir = tempfile::tempdir().expect("state directory");
        let first = StateFile::open(dir.path()).expect("first watcher");

        assert!(StateFile::open(dir.path()).is_err());

        drop(first);

        StateFile::open(dir.path()).expect("lock released when watcher exits");
    }

    #[test]
    fn reprocess_resets_outcomes_without_remigrating_legacy_discovery() {
        let dir = tempfile::tempdir().expect("state directory");
        fs::write(
            dir.path().join("processed-v3.json"),
            br#"{"entries":{"Persistent.Mail.9":{"size":1,"modified":2}}}"#,
        )
        .expect("legacy discovery");
        let file = StateFile::open(dir.path()).expect("state file");
        let mut store = file.load().expect("migrate");
        store.acknowledged.insert("123".to_string());
        store.rejected.insert(
            "Persistent.Mail.456".to_string(),
            RejectedUpload { sig: None, reason: "unsupported".to_string() },
        );
        file.save(&store).expect("save outcomes");

        file.reset().expect("explicit reprocess");
        drop(file);

        let file = StateFile::open(dir.path()).expect("restart");
        let restored = file.load().expect("reset state");

        assert!(restored.pending.is_empty());
        assert!(restored.acknowledged.is_empty());
        assert!(restored.rejected.is_empty());
        assert!(dir.path().join("processed-v3.json").exists());
    }
}
