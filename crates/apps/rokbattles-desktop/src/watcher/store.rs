//! Persists upload history and pending sources in SQLite.
//!
//! [`UploadDb`] holds an exclusive watcher lock for its lifetime. Commits use
//! write-ahead logging with full synchronization and update only changed records.
//! Recovery reads `local.db`; legacy JSON history is not imported.
//!
//! Captured bytes and their pending source updates share a transaction. Payloads
//! are removed after acknowledgement, permanent rejection, or removal of their
//! last pending source. Idle cleanup reclaims unused pages. Existing databases
//! enable this cleanup with a one-time rebuild before the watcher starts.
//!
//! Backlog limits apply to payload bytes and mail count; metadata and journals
//! require additional disk space. At capacity, capture leaves new sources pending
//! rather than evicting unuploaded payloads. Removing a watched directory cancels
//! its sources, releasing payloads that have no remaining pending copy.

use std::{
    cell::Cell,
    collections::BTreeMap,
    fs, io,
    path::Path,
    time::{Duration, SystemTime},
};

use anyhow::{Context, ensure};
use rusqlite::{Connection, params};
use rustc_hash::{FxHashMap, FxHashSet};
use tauri::{AppHandle, Manager};

use super::mail::mail_id_for_path;

mod migrations;

/// Maximum source and payload length. Keep the SQL payload constraint in sync.
pub(super) const MAX_MAIL_BYTES: u64 = 25 * 1024 * 1024;
/// Retained binary payload budget. Database metadata and journals are additional.
pub(super) const MAX_PREPARED_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// Bounds the scheduling metadata added by retained mail, including tiny payloads.
pub(super) const MAX_PREPARED_MAILS: usize = 200_000;

/// Upload history and pending work loaded from the database.
#[derive(Debug, Default)]
pub(crate) struct UploadStore {
    /// Mail IDs acknowledged by ingress, with leading zeroes preserved.
    pub(crate) acknowledged: FxHashSet<String>,
    /// Sources awaiting capture or upload, keyed by original path.
    pub(crate) pending: BTreeMap<String, QueuedUpload>,
    pub(crate) rejected: FxHashMap<String, RejectedUpload>,
    /// Retained payload lengths by mail ID. Bytes are read from SQLite only for upload.
    pub(crate) prepared: FxHashMap<String, u64>,
    /// API-wide retry deadline in milliseconds since the Unix epoch.
    pub(crate) api_backoff_until_ms: Option<u128>,
}

/// A source awaiting acknowledgement, including requests in progress.
#[derive(Debug, Clone)]
pub(crate) struct QueuedUpload {
    pub(crate) path: String,
    /// Last observed signature. Captured bytes may outlive this source path.
    pub(crate) sig: Option<FileSig>,
    /// Retry count used for exponential backoff, capped at 10.
    pub(crate) attempts: u32,
    /// Earliest attempt time in milliseconds since the Unix epoch.
    pub(crate) not_before_ms: Option<u128>,
}

#[derive(Debug)]
pub(crate) struct RejectedUpload {
    /// The rejected signature. A change allows discovery to enqueue the source again.
    pub(crate) sig: Option<FileSig>,
}

/// File metadata used to detect source changes.
///
/// Equal signatures do not guarantee identical contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileSig {
    /// File length in bytes.
    pub(crate) size: u64,
    /// Modification time in milliseconds since the Unix epoch.
    pub(crate) modified: u128,
}

/// A database update to apply in order with the other changes in a commit.
pub(crate) enum StoreChange {
    Pending(QueuedUpload),
    Rejected { path: String, sig: Option<FileSig> },
    Remove(String),
    Acknowledge(String),
    Backoff(Option<u128>),
    Prepared { mail_id: String, bytes: Vec<u8> },
}

/// A database connection that holds the exclusive watcher lock until dropped.
pub(crate) struct UploadDb {
    // Field order ensures the connection closes before the watcher lock is released.
    connection: Connection,
    _lock: fs::File,
    // A reader can postpone checkpointing freed pages; retry on later idle turns.
    cleanup_checkpoint_pending: Cell<bool>,
}

impl UploadDb {
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

        let mut connection = Connection::open(dir.join("local.db"))
            .context("Failed to open local upload database")?;
        initialize(&mut connection)?;

        Ok(Self { connection, _lock: lock, cleanup_checkpoint_pending: Cell::new(true) })
    }

    pub(crate) fn load(&self) -> anyhow::Result<UploadStore> {
        load_store(&self.connection)
    }

    /// Reads a retained payload without reopening the game's source file.
    ///
    /// Returns an error if the payload is missing, unreadable, or outside the size limit.
    pub(crate) fn read_prepared(&self, mail_id: &str) -> anyhow::Result<Vec<u8>> {
        let bytes: Vec<u8> = self
            .connection
            .prepare_cached("SELECT bytes FROM prepared_mail WHERE mail_id = ?1")?
            .query_row([mail_id], |row| row.get(0))?;
        ensure!(
            !bytes.is_empty() && bytes.len() as u64 <= MAX_MAIL_BYTES,
            "Invalid retained mail size"
        );

        Ok(bytes)
    }

    /// Reclaims at most 1,024 unused database pages, returning whether any work was needed.
    pub(crate) fn reclaim_unused_space(&self) -> anyhow::Result<bool> {
        let pages: u32 =
            self.connection.pragma_query_value(None, "freelist_count", |row| row.get(0))?;

        if pages == 0 {
            if self.cleanup_checkpoint_pending.get() {
                self.checkpoint_cleanup()?;
            }
            return Ok(false);
        }

        let mut statement = self.connection.prepare_cached("PRAGMA incremental_vacuum(1024)")?;
        let mut rows = statement.query([])?;

        // SQLite advances incremental vacuum once per result row. Execute the full page budget.
        while rows.next()?.is_some() {}

        let remaining: u32 =
            self.connection.pragma_query_value(None, "freelist_count", |row| row.get(0))?;
        ensure!(remaining < pages, "Incremental database cleanup made no progress");
        self.cleanup_checkpoint_pending.set(true);
        self.checkpoint_cleanup()?;

        Ok(true)
    }

    fn checkpoint_cleanup(&self) -> anyhow::Result<()> {
        let (frames, checkpointed): (i64, i64) =
            self.connection.query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| {
                Ok((row.get(1)?, row.get(2)?))
            })?;
        self.cleanup_checkpoint_pending.set(frames != checkpointed);

        Ok(())
    }

    /// Commits changes atomically in the supplied order.
    ///
    /// An empty batch performs no I/O.
    ///
    /// # Errors
    ///
    /// Returns an error if a record is invalid or the transaction fails. Retain the
    /// batch for retry or stop the watcher before starting another upload.
    pub(crate) fn commit(&self, changes: &[StoreChange]) -> anyhow::Result<()> {
        if changes.is_empty() {
            return Ok(());
        }

        // No transaction outlives this call, so the connection can be borrowed immutably.
        let transaction = self.connection.unchecked_transaction()?;

        for change in changes {
            apply_change(&transaction, change)?;
        }

        transaction.commit().context("Failed to commit upload state")
    }

    /// Clears history and retry delays, retaining captured sources for another upload attempt.
    ///
    /// Captured bytes may be the only remaining copy after game-side deletion.
    /// Uncaptured pending sources and rejections are rediscovered from watched directories.
    ///
    /// # Errors
    ///
    /// Returns an error if recovery state cannot be read or validated, or the reset
    /// transaction fails. Unreadable recovery state is left in place.
    pub(crate) fn reset(&self) -> anyhow::Result<()> {
        self.load()?;

        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute_batch(
            "DELETE FROM sources WHERE NOT EXISTS (
                 SELECT 1 FROM prepared_mail WHERE prepared_mail.mail_id = sources.mail_id
             );
             UPDATE sources SET pending = 1, attempts = 0, not_before_ms = NULL;
             DELETE FROM acknowledged;
             UPDATE upload_control SET api_backoff_until_ms = NULL WHERE id = 1;",
        )?;

        transaction.commit().context("Failed to reset upload history")
    }

    #[cfg(test)]
    pub(crate) fn read_for_test(dir: &Path) -> anyhow::Result<UploadStore> {
        let connection = Connection::open_with_flags(
            dir.join("local.db"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;

        load_store(&connection)
    }
}

fn initialize(connection: &mut Connection) -> anyhow::Result<()> {
    // Apply the durability setting before migrations write to the database.
    connection.pragma_update(None, "synchronous", "FULL")?;
    migrations::apply(connection)?;

    // Validate the schema before changing an existing database's journal mode.
    let mode: String =
        connection.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
    ensure!(mode.eq_ignore_ascii_case("wal"), "Upload database requires WAL mode");

    let vacuum_mode: u32 = connection.pragma_query_value(None, "auto_vacuum", |row| row.get(0))?;

    if vacuum_mode != 2 {
        connection.pragma_update(None, "auto_vacuum", "INCREMENTAL")?;

        if vacuum_mode == 0 {
            connection
                .execute_batch("VACUUM;")
                .context("Failed to enable incremental database cleanup")?;
        }
    }

    Ok(())
}

fn sqlite_integer(value: u128) -> anyhow::Result<i64> {
    i64::try_from(value).context("Upload state integer exceeds SQLite range")
}

fn apply_change(connection: &Connection, change: &StoreChange) -> anyhow::Result<()> {
    match change {
        StoreChange::Pending(item) => {
            put_source(
                connection,
                &item.path,
                item.sig.as_ref(),
                true,
                item.attempts,
                item.not_before_ms,
            )?;
        }
        StoreChange::Rejected { path, sig } => {
            put_source(connection, path, sig.as_ref(), false, 0, None)?;
            connection
                .prepare_cached("DELETE FROM prepared_mail WHERE mail_id = ?1")?
                .execute([mail_id_for_path(path)])?;
        }
        StoreChange::Remove(path) => {
            connection
                .prepare_cached(
                    "DELETE FROM prepared_mail
                 WHERE mail_id = (SELECT mail_id FROM sources WHERE path = ?1)
                 AND NOT EXISTS (
                     SELECT 1 FROM sources
                     WHERE sources.mail_id = prepared_mail.mail_id AND pending = 1 AND path <> ?1
                 )",
                )?
                .execute([path])?;
            connection.prepare_cached("DELETE FROM sources WHERE path = ?1")?.execute([path])?;
        }
        StoreChange::Acknowledge(id) => {
            ensure!(valid_mail_id(id), "Invalid acknowledged mail ID");

            connection
                .prepare_cached("INSERT OR IGNORE INTO acknowledged(mail_id) VALUES (?1)")?
                .execute([id])?;
            connection.prepare_cached("DELETE FROM sources WHERE mail_id = ?1")?.execute([id])?;
            connection
                .prepare_cached("DELETE FROM prepared_mail WHERE mail_id = ?1")?
                .execute([id])?;
        }
        StoreChange::Backoff(until) => {
            connection
                .prepare_cached("UPDATE upload_control SET api_backoff_until_ms = ?1 WHERE id = 1")?
                .execute([until.map(sqlite_integer).transpose()?])?;
        }
        StoreChange::Prepared { mail_id, bytes } => {
            ensure!(valid_mail_id(mail_id), "Invalid prepared mail ID");
            let inserted = connection
                .prepare_cached(
                    "INSERT INTO prepared_mail(mail_id, bytes)
                 SELECT ?1, ?2 WHERE EXISTS (
                     SELECT 1 FROM sources WHERE mail_id = ?1 AND pending = 1
                 )",
                )?
                .execute(params![mail_id, bytes])?;
            ensure!(inserted == 1, "Prepared mail has no pending source");
        }
    }

    Ok(())
}

fn put_source(
    connection: &Connection,
    path: &str,
    sig: Option<&FileSig>,
    pending: bool,
    attempts: u32,
    not_before: Option<u128>,
) -> anyhow::Result<()> {
    let id = mail_id_for_path(path);
    let size = sig.map(|sig| sqlite_integer(u128::from(sig.size))).transpose()?;
    let modified = sig.map(|sig| sqlite_integer(sig.modified)).transpose()?;
    let not_before = not_before.map(sqlite_integer).transpose()?;

    connection
        .prepare_cached(
            "INSERT INTO sources(path, mail_id, pending, size, modified_ms, attempts, not_before_ms)
             SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7
             WHERE NOT EXISTS (SELECT 1 FROM acknowledged WHERE mail_id = ?2)
             ON CONFLICT(path) DO UPDATE SET
                mail_id = excluded.mail_id,
                pending = excluded.pending,
                size = excluded.size,
                modified_ms = excluded.modified_ms,
                attempts = excluded.attempts,
                not_before_ms = excluded.not_before_ms",
        )?
        .execute(params![path, id, pending, size, modified, attempts, not_before])?;

    Ok(())
}

fn valid_mail_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()) && id.parse::<u128>().is_ok()
}

fn read_unsigned(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<Option<u64>> {
    row.get::<_, Option<i64>>(index)?.map(u64::try_from).transpose().map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}

fn load_store(connection: &Connection) -> anyhow::Result<UploadStore> {
    // Use one snapshot so sources and acknowledgements cannot come from different commits.
    let transaction = connection.unchecked_transaction()?;
    let mut store = UploadStore::default();

    {
        let mut statement = transaction.prepare("SELECT mail_id FROM acknowledged")?;

        for id in statement.query_map([], |row| row.get::<_, String>(0))? {
            let id = id?;
            ensure!(valid_mail_id(&id), "Invalid acknowledged mail ID; database retained");

            store.acknowledged.insert(id);
        }
    }

    {
        let mut statement = transaction.prepare(
            "SELECT path, mail_id, pending, size, modified_ms, attempts, not_before_ms
             FROM sources ORDER BY rowid",
        )?;
        let mut rows = statement.query([])?;

        while let Some(row) = rows.next()? {
            let path: String = row.get(0)?;
            let id: Option<String> = row.get(1)?;

            ensure!(
                id.as_deref() == mail_id_for_path(&path),
                "Invalid source mail ID; database retained"
            );
            ensure!(
                !id.as_ref().is_some_and(|id| store.acknowledged.contains(id)),
                "Source is already acknowledged; database retained"
            );

            let sig = match (read_unsigned(row, 3)?, read_unsigned(row, 4)?) {
                (Some(size), Some(modified)) => {
                    Some(FileSig { size, modified: u128::from(modified) })
                }
                (None, None) => None,
                _ => anyhow::bail!("Incomplete source signature; database retained"),
            };

            if row.get::<_, bool>(2)? {
                store.pending.insert(
                    path.clone(),
                    QueuedUpload {
                        path,
                        sig,
                        attempts: row.get(5)?,
                        not_before_ms: read_unsigned(row, 6)?.map(u128::from),
                    },
                );
            } else {
                store.rejected.insert(path, RejectedUpload { sig });
            }
        }
    }

    {
        let pending_ids: FxHashSet<_> =
            store.pending.keys().filter_map(|path| mail_id_for_path(path)).collect();
        let mut statement =
            transaction.prepare("SELECT mail_id, length(bytes) FROM prepared_mail")?;

        for entry in
            statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?
        {
            let (id, size) = entry?;
            let size = u64::try_from(size).context("Invalid prepared mail size")?;
            ensure!(
                pending_ids.contains(id.as_str()),
                "Prepared mail has no pending source; database retained"
            );
            ensure!(
                size > 0 && size <= MAX_MAIL_BYTES,
                "Invalid prepared mail size; database retained"
            );
            store.prepared.insert(id, size);
        }
    }

    store.api_backoff_until_ms = transaction
        .query_row("SELECT api_backoff_until_ms FROM upload_control WHERE id = 1", [], |row| {
            read_unsigned(row, 0)
        })?
        .map(u128::from);
    transaction.commit()?;

    Ok(store)
}

pub(crate) fn file_sig(meta: &fs::Metadata) -> anyhow::Result<FileSig> {
    let modified = meta
        .modified()
        .context("Could not read file modification time")?
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis();

    Ok(FileSig { size: meta.len(), modified })
}

/// Returns whether an absolute path is missing and its parent directory is readable.
pub(super) fn source_is_deleted(path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }

    if !fs::symlink_metadata(path).is_err_and(|error| error.kind() == io::ErrorKind::NotFound) {
        return false;
    }

    path.parent().is_some_and(|parent| {
        fs::read_dir(parent)
            .is_ok_and(|mut entries| entries.next().is_none_or(|entry| entry.is_ok()))
    })
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    fn pending(path: &str) -> QueuedUpload {
        QueuedUpload {
            path: path.to_string(),
            sig: Some(FileSig { size: 100, modified: 200 }),
            attempts: 2,
            not_before_ms: Some(4000),
        }
    }

    #[test]
    fn acknowledged_payload_space_can_be_returned_to_the_filesystem_in_bounded_steps() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");
        db.commit(&[
            StoreChange::Pending(pending("Persistent.Mail.1")),
            StoreChange::Prepared { mail_id: "1".to_string(), bytes: vec![7; 8 * 1024 * 1024] },
        ])
        .expect("durable payload");
        db.connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
            .expect("checkpoint payload");
        let file_before = fs::metadata(dir.path().join("local.db")).expect("allocated file").len();
        let before: u32 = db
            .connection
            .pragma_query_value(None, "page_count", |row| row.get(0))
            .expect("allocated pages");
        db.commit(&[StoreChange::Acknowledge("1".to_string())]).expect("acknowledgement");
        let mut steps = 0;

        while db.reclaim_unused_space().expect("incremental cleanup") {
            steps += 1;
            assert!(steps < 32, "cleanup must make progress");
        }

        let after: u32 = db
            .connection
            .pragma_query_value(None, "page_count", |row| row.get(0))
            .expect("remaining pages");
        assert!(after < before / 2, "deleted payload pages should not remain allocated forever");
        assert!(db.load().expect("remaining history").acknowledged.contains("1"));
        assert!(!db.reclaim_unused_space().expect("idle cleanup"));
        let file_after = fs::metadata(dir.path().join("local.db")).expect("compacted file").len();
        assert!(file_after < file_before / 2, "cleanup must release on-disk allocation too");
    }

    #[test]
    fn orphan_payload_blocks_loading_and_reset_without_deleting_recovery_data() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");
        db.connection
            .execute("INSERT INTO prepared_mail VALUES ('1', ?1)", [b"orphan bytes".as_slice()])
            .expect("inconsistent recovery fixture");

        db.load().expect_err("a payload must have a pending source");
        db.reset().expect_err("reset must retain unreadable recovery state");
        assert_eq!(db.read_prepared("1").expect("recovery data remains"), b"orphan bytes");
    }

    #[test]
    fn cleanup_retries_a_checkpoint_delayed_by_a_reader_without_a_busy_loop() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");
        db.commit(&[
            StoreChange::Pending(pending("Persistent.Mail.1")),
            StoreChange::Prepared { mail_id: "1".to_string(), bytes: vec![7; 1024 * 1024] },
        ])
        .expect("payload");
        db.connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
            .expect("checkpoint payload");
        let before = fs::metadata(dir.path().join("local.db")).expect("database size").len();
        let reader = Connection::open(dir.path().join("local.db")).expect("reader");
        let snapshot = reader.unchecked_transaction().expect("read snapshot");
        let bytes: i64 = snapshot
            .query_row("SELECT length(bytes) FROM prepared_mail", [], |row| row.get(0))
            .expect("snapshot payload");
        db.commit(&[StoreChange::Acknowledge("1".to_string())]).expect("acknowledgement");
        assert!(db.reclaim_unused_space().expect("cleanup while reader is active"));
        let snapshot_bytes: i64 = snapshot
            .query_row("SELECT length(bytes) FROM prepared_mail", [], |row| row.get(0))
            .expect("reader keeps its snapshot");

        assert_eq!(snapshot_bytes, bytes);
        snapshot.commit().expect("release reader");
        assert!(
            !db.reclaim_unused_space().expect("retry delayed checkpoint"),
            "checkpoint retries should allow the watcher to sleep"
        );
        let after =
            fs::metadata(dir.path().join("local.db")).expect("compacted database size").len();
        assert!(after < before / 2, "the deferred checkpoint must eventually release disk space");
    }

    #[test]
    fn version_one_pending_sources_survive_enabling_payload_storage_and_cleanup() {
        let dir = tempfile::tempdir().expect("state directory");
        let connection = Connection::open(dir.path().join("local.db")).expect("legacy database");
        connection
            .execute_batch(include_str!("../../migrations/0001_initial.sql"))
            .expect("version one schema");
        connection.pragma_update(None, "user_version", 1).expect("old version");
        connection
            .execute_batch(
                "INSERT INTO sources VALUES ('Persistent.Mail.001', '001', 1, 100, 200, 2, 4000);",
            )
            .expect("legacy retry");
        drop(connection);
        let db = UploadDb::open(dir.path()).expect("upgrade payload storage");
        let restored = db.load().expect("recovered sources");
        let source = restored.pending.get("Persistent.Mail.001").expect("legacy source");

        assert_eq!(source.attempts, 2);
        assert_eq!(source.not_before_ms, Some(4000));
        assert!(restored.prepared.is_empty());
        let mode: u32 = db
            .connection
            .pragma_query_value(None, "auto_vacuum", |row| row.get(0))
            .expect("cleanup mode");
        assert_eq!(mode, 2);
    }

    #[test]
    fn sql_payload_size_limit_matches_the_source_reader_limit() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");
        db.commit(&[StoreChange::Pending(pending("Persistent.Mail.1"))]).expect("source");
        let maximum = usize::try_from(MAX_MAIL_BYTES).expect("supported source size");
        db.commit(&[StoreChange::Prepared { mail_id: "1".to_string(), bytes: vec![7; maximum] }])
            .expect("maximum payload is accepted");
        db.commit(&[
            StoreChange::Acknowledge("1".to_string()),
            StoreChange::Pending(pending("Persistent.Mail.2")),
        ])
        .expect("release first payload");
        let error = db
            .commit(&[StoreChange::Prepared {
                mail_id: "2".to_string(),
                bytes: vec![7; maximum + 1],
            }])
            .expect_err("oversized payload is rejected");

        assert!(error.to_string().contains("CHECK constraint failed"));
        assert!(
            db.load().expect("durable source remains").pending.contains_key("Persistent.Mail.2")
        );
    }

    #[test]
    fn fresh_database_ignores_all_legacy_history_including_corrupt_json() {
        let dir = tempfile::tempdir().expect("state directory");
        let legacy =
            ["processed-v3.json", "processed-v3.tmp", "upload-queue.json", "upload-state-v1.json"];

        for name in legacy {
            fs::write(dir.path().join(name), b"not imported, even if corrupt")
                .expect("legacy file");
        }

        let db = UploadDb::open(dir.path()).expect("fresh database");
        let store = db.load().expect("fresh history");

        assert!(store.acknowledged.is_empty());
        assert!(store.pending.is_empty());
        assert!(store.rejected.is_empty());
        assert!(store.api_backoff_until_ms.is_none());

        for name in legacy {
            assert_eq!(
                fs::read(dir.path().join(name)).expect("legacy file retained"),
                b"not imported, even if corrupt"
            );
        }
    }

    #[test]
    fn transaction_round_trips_only_the_state_needed_for_recovery() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");

        db.commit(&[
            StoreChange::Pending(pending("mail/Persistent.Mail.001")),
            StoreChange::Rejected {
                path: "mail/Persistent.Mail.002".into(),
                sig: Some(FileSig { size: 3, modified: 4 }),
            },
            StoreChange::Acknowledge("003".into()),
            StoreChange::Backoff(Some(9000)),
        ])
        .expect("commit records");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart");
        let store = db.load().expect("recover records");
        let item = store.pending.values().next().expect("pending source");

        assert_eq!(item.path, "mail/Persistent.Mail.001");
        assert_eq!(item.sig, Some(FileSig { size: 100, modified: 200 }));
        assert_eq!(item.attempts, 2);
        assert_eq!(item.not_before_ms, Some(4000));
        assert_eq!(
            store.rejected["mail/Persistent.Mail.002"].sig,
            Some(FileSig { size: 3, modified: 4 })
        );
        assert!(store.acknowledged.contains("003"));
        assert!(!store.acknowledged.contains("3"));
        assert_eq!(store.api_backoff_until_ms, Some(9000));
        assert!(!dir.path().join("upload-state-v1.json").exists());
    }

    #[test]
    fn acknowledgement_removes_all_copies_and_preserves_distinct_leading_zero_ids() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");

        db.commit(&[
            StoreChange::Pending(pending("first/Persistent.Mail.1")),
            StoreChange::Rejected { path: "second/Persistent.Mail.1".into(), sig: None },
            StoreChange::Pending(pending("first/Persistent.Mail.01")),
            StoreChange::Acknowledge("1".into()),
            StoreChange::Pending(pending("late/Persistent.Mail.1")),
        ])
        .expect("acknowledge all exact copies");
        let store = db.load().expect("acknowledged state");

        assert_eq!(store.pending.len(), 1);
        assert_eq!(
            store.pending.values().next().expect("distinct ID").path,
            "first/Persistent.Mail.01"
        );
        assert!(store.rejected.is_empty());
    }

    #[test]
    fn failed_transaction_rolls_back_all_earlier_changes() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");
        db.commit(&[StoreChange::Pending(pending("Persistent.Mail.1"))])
            .expect("original pending source");

        let mut invalid = pending("Persistent.Mail.2");
        invalid.not_before_ms = Some(u128::MAX);

        assert!(
            db.commit(&[StoreChange::Acknowledge("1".into()), StoreChange::Pending(invalid)])
                .is_err()
        );

        let store = db.load().expect("previous committed state");

        assert!(store.acknowledged.is_empty());
        assert_eq!(
            store.pending.values().next().expect("original source").path,
            "Persistent.Mail.1"
        );
    }

    #[test]
    fn retry_update_writes_bounded_pages_with_large_history_and_idle_commits_write_nothing() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");
        let history: Vec<_> =
            (1_000_000..1_050_000).map(|id| StoreChange::Acknowledge(id.to_string())).collect();
        db.commit(&history).expect("large acknowledged history");

        let mut item = pending("mail/Persistent.Mail.1");
        db.commit(&[StoreChange::Pending(item.clone())]).expect("pending source");
        db.connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .expect("baseline checkpoint");
        let database_bytes =
            fs::metadata(dir.path().join("local.db")).expect("database size").len();

        item.attempts += 1;
        item.not_before_ms = Some(8000);
        db.commit(&[StoreChange::Pending(item)]).expect("single retry update");

        let wal = dir.path().join("local.db-wal");
        let written = fs::metadata(&wal).expect("changed pages").len();

        assert!(written > 0);
        assert!(written < 64 * 1024, "single retry wrote {written} bytes");
        assert!(
            written * 32 < database_bytes,
            "retry must not rewrite {database_bytes} bytes of history"
        );

        let before = fs::metadata(&wal).expect("before idle");

        for _ in 0..100 {
            db.commit(&[]).expect("idle");
        }

        let after = fs::metadata(&wal).expect("after idle");

        assert_eq!(before.len(), after.len());
        assert_eq!(
            before.modified().expect("before mtime"),
            after.modified().expect("after mtime")
        );
    }

    #[test]
    fn reset_clears_history_and_retries_without_reimporting_legacy_files() {
        let dir = tempfile::tempdir().expect("state directory");
        fs::write(dir.path().join("processed-v3.json"), b"legacy remains ignored").expect("legacy");
        let db = UploadDb::open(dir.path()).expect("database");
        db.commit(&[
            StoreChange::Pending(pending("Persistent.Mail.1")),
            StoreChange::Rejected { path: "Persistent.Mail.2".into(), sig: None },
            StoreChange::Acknowledge("3".into()),
            StoreChange::Backoff(Some(1234)),
        ])
        .expect("history");

        db.reset().expect("reprocess");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart");
        let store = db.load().expect("cleared history");

        assert!(
            store.pending.is_empty() && store.rejected.is_empty() && store.acknowledged.is_empty()
        );
        assert!(store.api_backoff_until_ms.is_none());
    }

    #[test]
    fn failed_reset_preserves_all_recovery_state() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");
        db.commit(&[
            StoreChange::Pending(pending("Persistent.Mail.1")),
            StoreChange::Rejected { path: "Persistent.Mail.2".into(), sig: None },
            StoreChange::Acknowledge("3".into()),
            StoreChange::Backoff(Some(9000)),
        ])
        .expect("recovery state");
        db.connection
            .execute_batch(
                "CREATE TRIGGER fail_reset BEFORE DELETE ON acknowledged
                 BEGIN
                    SELECT RAISE(ABORT, 'injected reset failure');
                 END;",
            )
            .expect("fail after sources have been deleted");

        db.reset().expect_err("reset must roll back");
        drop(db);

        let db = UploadDb::open(dir.path()).expect("restart after failed reset");
        let store = db.load().expect("recovery state retained");
        let pending = store.pending.values().next().expect("pending source retained");

        assert_eq!(pending.path, "Persistent.Mail.1");
        assert_eq!(pending.attempts, 2);
        assert_eq!(pending.not_before_ms, Some(4000));
        assert!(store.rejected.contains_key("Persistent.Mail.2"));
        assert!(store.acknowledged.contains("3"));
        assert_eq!(store.api_backoff_until_ms, Some(9000));
    }

    #[test]
    fn corrupt_database_and_unsupported_schema_are_preserved() {
        let dir = tempfile::tempdir().expect("state directory");
        let path = dir.path().join("local.db");
        fs::write(&path, b"not a SQLite database").expect("corrupt database");

        assert!(UploadDb::open(dir.path()).is_err());
        assert_eq!(fs::read(&path).expect("corrupt file retained"), b"not a SQLite database");

        fs::remove_file(&path).expect("new test fixture");
        let connection = Connection::open(&path).expect("newer database");
        connection.pragma_update(None, "user_version", i32::MAX).expect("newer schema");
        drop(connection);
        let before = fs::read(&path).expect("original database");

        assert!(UploadDb::open(dir.path()).is_err());
        assert_eq!(fs::read(&path).expect("newer database retained"), before);
    }

    #[test]
    fn contradictory_state_blocks_loading_and_reset_without_erasing_recovery() {
        let dir = tempfile::tempdir().expect("state directory");
        let db = UploadDb::open(dir.path()).expect("database");
        db.commit(&[StoreChange::Pending(pending("Persistent.Mail.1"))]).expect("pending source");

        db.connection
            .execute("INSERT INTO acknowledged VALUES ('1')", [])
            .expect("inject logical corruption");

        db.load().expect_err("contradictory recovery data must be rejected");
        assert!(db.reset().is_err());

        let count: u32 = db
            .connection
            .query_row("SELECT count(*) FROM sources", [], |row| row.get(0))
            .expect("source retained");

        assert_eq!(count, 1);
    }

    #[test]
    fn only_one_watcher_can_own_the_database() {
        let dir = tempfile::tempdir().expect("state directory");
        let first = UploadDb::open(dir.path()).expect("first watcher");

        assert!(UploadDb::open(dir.path()).is_err());

        drop(first);
        UploadDb::open(dir.path()).expect("ownership released");
    }

    #[test]
    fn committed_wal_recovers_after_process_exit_without_destructors() {
        let dir = tempfile::tempdir().expect("state directory");

        let status = Command::new(std::env::current_exe().expect("test executable"))
            .args(["--exact", "watcher::store::tests::crash_writer", "--nocapture"])
            .env("ROKBATTLES_TEST_CRASH_DIR", dir.path())
            .status()
            .expect("crash fixture process");

        assert!(status.success());
        assert!(dir.path().join("local.db-wal").exists(), "fixture must leave uncheckpointed WAL");

        let db = UploadDb::open(dir.path()).expect("recover after abrupt exit");
        let store = db.load().expect("committed WAL recovered");

        assert!(store.acknowledged.contains("1"));
        assert_eq!(
            store.pending.values().next().expect("pending source").path,
            "Persistent.Mail.2"
        );
        assert_eq!(store.api_backoff_until_ms, Some(9000));
    }

    #[test]
    fn crash_writer() {
        let Some(dir) = std::env::var_os("ROKBATTLES_TEST_CRASH_DIR") else {
            return;
        };

        let db = UploadDb::open(Path::new(&dir)).expect("child database");
        db.commit(&[
            StoreChange::Acknowledge("1".into()),
            StoreChange::Pending(pending("Persistent.Mail.2")),
            StoreChange::Backoff(Some(9000)),
        ])
        .expect("commit before abrupt exit");

        std::process::exit(0);
    }
}
