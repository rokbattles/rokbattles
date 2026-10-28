//! Buffer decoded mail, commit it to disk, and retry delivery to relay ingress.
//!
//! [`Writer`] normally queues work for a dedicated disk thread. Adjacent queued
//! batches with equal player/server context are coalesced within ingress limits.
//! Queue saturation falls back to a synchronous disk commit instead of immediately
//! discarding the batch; this can delay passive capture under prolonged overload.
//!
//! # Durability boundary
//!
//! Queue acceptance is volatile. A disk commit writes and syncs a temporary file,
//! renames it to `.batch`, then syncs the directory. Only `.batch` files are upload
//! candidates. Startup removes unfinished `.tmp` files. Process exit before commit
//! can lose queued mail; disk-budget or I/O failures are logged and may lose mail.
//!
//! Upload failures retain the committed file. A valid acknowledgement removes
//! the whole batch, including entries rejected by ingress. If removal or its
//! directory sync fails, a batch may be retried; ingress must tolerate duplicates.
//!
//! # Bounds and ownership
//!
//! The memory queue holds at most 8192 batches and charges at most 128 MiB of
//! payload lengths and batch metadata. This charge excludes allocator overhead,
//! larger backing allocations shared by `Bytes`, and the disk writer's current
//! batch. A batch contains at most 512 entries and 24 MiB of payload. The disk
//! spool allows at most 8192 committed files and 512 MiB of encoded records.
//!
//! Open the private state directory once, then clone [`Spool`] to share its commit
//! lock. Independently opening the same directory is not coordinated: startup
//! cleanup could remove another writer's temporary file.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use bytes::Bytes;
use rokbattles_gateway_protocol::uploader::{MailBatch, MailContext, UploadError};

const MAX_BATCH: usize = 24 * 1024 * 1024;
const MAX_SPOOL: u64 = 512 * 1024 * 1024;
const MAX_FILES: usize = 8192;
const MAX_QUEUED_BATCHES: usize = 8192;
const MAX_QUEUED_BYTES: usize = 128 * 1024 * 1024;

static SERIAL: AtomicU64 = AtomicU64::new(0);

/// Producer handle for the spool's bounded background writer.
///
/// Obtain one from [`Spool::spawn_writer`]. Successful submission may still be
/// in memory; only [`Spool::submit`] confirms a synchronous disk commit.
pub struct Writer {
    sender: SyncSender<QueuedBatch>,
    queued_bytes: Arc<AtomicUsize>,
    max_bytes: usize,
    spool: Spool,
}

// The reservation follows a batch through channel send failures and pending
// lookahead. Dropping it releases the charge even when the receiver disconnects.
pub(crate) struct QueuedBatch {
    batch: MailBatch,
    _reservation: Reservation,
}

struct Reservation {
    queued_bytes: Arc<AtomicUsize>,
    bytes: usize,
}

impl Drop for Reservation {
    fn drop(&mut self) {
        // This counter only accounts for bytes. The channel, not this atomic,
        // synchronizes access to the payload, so relaxed ordering is sufficient.
        self.queued_bytes.fetch_sub(self.bytes, Ordering::Relaxed);
    }
}

impl QueuedBatch {
    // Release the queue charge as ownership moves into the disk writer or fallback.
    pub(crate) fn into_batch(self) -> MailBatch {
        self.batch
    }
}

impl Writer {
    pub(crate) fn channel(
        spool: Spool,
        capacity: usize,
        max_bytes: usize,
    ) -> (Self, Receiver<QueuedBatch>) {
        let (sender, receiver) = mpsc::sync_channel(capacity);
        let writer = Self { sender, queued_bytes: Arc::new(AtomicUsize::new(0)), max_bytes, spool };
        (writer, receiver)
    }

    /// Accept a batch into memory, or synchronously commit it if queuing fails.
    ///
    /// Normal sends do not wait for disk I/O. Reaching either queue limit or
    /// losing the receiver invokes [`Spool::submit`] on the caller's thread;
    /// despite the name, this fallback can block on the commit lock and filesystem.
    /// It does not block kernel forwarding, but delayed capture may lose packets.
    ///
    /// `Ok(())` means either queued or committed, not necessarily durable. Batches
    /// must satisfy the spool format limits; validation of queued batches occurs
    /// on the writer thread, and commit failures there are logged separately.
    ///
    /// # Errors
    ///
    /// Returns [`Spool::submit`] errors when the synchronous fallback fails.
    /// A failure after rename may still leave a committed file in the directory.
    pub fn try_send(&self, batch: MailBatch) -> io::Result<()> {
        let bytes = batch.entries.iter().map(Bytes::len).sum::<usize>()
            + batch.entries.capacity() * std::mem::size_of::<Bytes>()
            + std::mem::size_of::<MailBatch>();
        if self
            .queued_bytes
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                used.checked_add(bytes).filter(|total| *total <= self.max_bytes)
            })
            .is_ok()
        {
            let queued = QueuedBatch {
                batch,
                _reservation: Reservation { queued_bytes: self.queued_bytes.clone(), bytes },
            };
            match self.sender.try_send(queued) {
                Ok(()) => return Ok(()),
                Err(TrySendError::Full(queued) | TrySendError::Disconnected(queued)) => {
                    let batch = queued.into_batch();
                    tracing::warn!(
                        entries = batch.entries.len(),
                        "upload writer unavailable or queue full; committing batch directly to disk"
                    );
                    return self.spool.submit(&batch);
                }
            }
        }
        tracing::warn!(
            entries = batch.entries.len(),
            "upload memory budget reached; committing batch directly to disk"
        );
        self.spool.submit(&batch)
    }
}

/// Shared access to one worker-owned directory of committed upload batches.
///
/// Clones share a lock covering disk-budget inspection and file commit. Upload
/// deletion may proceed concurrently; budget accounting tolerates files removed
/// between listing and metadata lookup. Use [`crate::service::validate_spool`]
/// before opening the production directory.
#[derive(Clone)]
pub struct Spool {
    path: PathBuf,
    commit_lock: Arc<Mutex<()>>,
}

impl Spool {
    /// Start a disk writer and return its bounded producer handle.
    ///
    /// Queues up to 8192 batches with a 128 MiB accounting budget. The writer
    /// coalesces adjacent batches only when their player/server context matches
    /// and the result fits the entry-count and payload limits. It does not wait
    /// for more input merely to fill a batch.
    ///
    /// Dropping the handle closes its sender; the detached thread drains queued
    /// work while the process remains alive. There is no join or process-shutdown
    /// durability guarantee. Disk commit failures are logged, not returned here.
    ///
    /// # Panics
    ///
    /// Panics if the OS cannot create the writer thread.
    pub fn spawn_writer(&self) -> Writer {
        let (writer, receiver) =
            Writer::channel(self.clone(), MAX_QUEUED_BATCHES, MAX_QUEUED_BYTES);
        let spool = self.clone();
        std::thread::spawn(move || spool.write_loop(receiver));
        tracing::info!(
            max_batches = MAX_QUEUED_BATCHES,
            max_bytes = MAX_QUEUED_BYTES,
            "upload burst buffer configured"
        );
        writer
    }

    /// Drain queued mail, preserving context and ingress limits when coalescing.
    fn write_loop(&self, receiver: Receiver<QueuedBatch>) {
        // Keep one incompatible batch for the next commit rather than losing it
        // or merging mail that needs a different reconstruction context.
        let mut pending = None;

        loop {
            let Some(queued) = pending.take().or_else(|| receiver.recv().ok()) else {
                return;
            };

            let mut batch = queued.into_batch();
            let mut bytes = batch.entries.iter().map(Bytes::len).sum::<usize>();
            while let Ok(queued) = receiver.try_recv() {
                let next_bytes = queued.batch.entries.iter().map(Bytes::len).sum::<usize>();
                if queued.batch.context != batch.context
                    || batch.entries.len() + queued.batch.entries.len() > 512
                    || bytes + next_bytes > MAX_BATCH
                {
                    pending = Some(queued);
                    break;
                }
                bytes += next_bytes;
                batch.entries.extend(queued.into_batch().entries);
            }

            if let Err(error) = self.submit(&batch) {
                tracing::error!(%error, entries = batch.entries.len(), "mail batch not committed; observation remains active");
            }
        }
    }

    /// Create the spool directory if needed and remove unfinished `.tmp` files.
    ///
    /// Existing `.batch` files are retained for retry. The caller must establish
    /// directory ownership/permissions and exclusive startup access; this method
    /// does not enforce the production directory policy.
    ///
    /// # Errors
    ///
    /// Returns directory creation, listing, or temporary-file removal errors.
    pub fn open(path: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&path)?;
        let spool = Self { path, commit_lock: Arc::new(Mutex::new(())) };
        // Temporary names never enter the upload scan, even after a process crash.
        for entry in fs::read_dir(&spool.path)? {
            let path = entry?.path();
            if path.extension().is_some_and(|ext| ext == "tmp") {
                fs::remove_file(path)?;
            }
        }
        Ok(spool)
    }

    /// List `.batch` paths in lexicographic filename order.
    ///
    /// Names normally reflect commit order, but timestamps can move backward
    /// with the system clock. A listed file may be removed before it is read.
    /// File type and contents are validated by [`Self::read`], not this listing.
    ///
    /// # Errors
    ///
    /// Returns directory listing or entry-access errors.
    pub fn pending(&self) -> io::Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        for entry in fs::read_dir(&self.path)? {
            let path = entry?.path();
            if path.extension().is_some_and(|ext| ext == "batch") {
                paths.push(path);
            }
        }
        paths.sort();
        Ok(paths)
    }

    /// Encode and synchronously commit a batch within the shared disk budget.
    ///
    /// Accepts 1–512 raw entries totaling at most 24 MiB of payload. Encoded records
    /// count toward the 512 MiB/8192-file spool limits. The commit lock serializes
    /// budget inspection and writes across clones, including queue fallback.
    ///
    /// # Errors
    ///
    /// Returns invalid-batch, poisoned-lock, full-spool, or filesystem errors.
    /// Success follows file sync, atomic rename, and directory sync. An error
    /// after rename can leave a valid `.batch` file; callers must allow retries
    /// to produce duplicate uploads rather than assume nothing was persisted.
    pub fn submit(&self, batch: &MailBatch) -> io::Result<()> {
        let bytes = encode(batch)?;
        // Serialize the budget check with commit so concurrent fallbacks cannot overbook it.
        let _commit = self
            .commit_lock
            .lock()
            .map_err(|_error| io::Error::other("spool writer lock poisoned"))?;

        let pending = self.pending()?;
        let total =
            pending.iter().try_fold(0_u64, |sum, path| match fs::symlink_metadata(path) {
                Ok(metadata) => Ok(sum.saturating_add(metadata.len())),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(sum),
                Err(error) => Err(error),
            })?;
        if pending.len() >= MAX_FILES || total.saturating_add(bytes.len() as u64) > MAX_SPOOL {
            return Err(io::Error::other("upload spool full"));
        }

        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let path = self
            .path
            .join(format!("{stamp:032}-{:016}.tmp", SERIAL.fetch_add(1, Ordering::Relaxed)));
        let result = (|| {
            let mut file = OpenOptions::new().write(true).create_new(true).open(&path)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&path, path.with_extension("batch"))?;
            File::open(&self.path)?.sync_all()
        })();
        if result.is_err() {
            let _cleanup = fs::remove_file(&path);
        }
        result
    }

    /// Read and decode a bounded, regular spool file without following symlinks.
    ///
    /// `O_NOFOLLOW` rejects a symlink at the final path component. `O_NONBLOCK`
    /// avoids waiting on a FIFO before its type can be checked. Encoded length
    /// is checked both before and after reading to bound a file that grows.
    ///
    /// # Errors
    ///
    /// Returns open/read errors or rejects non-regular files, oversized records,
    /// unsupported format versions, invalid headers, truncated entries, or trailing
    /// bytes. Protobuf entry contents are left to ingress validation.
    pub fn read(path: &Path) -> io::Result<MailBatch> {
        use std::os::unix::fs::OpenOptionsExt;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)?;
        let metadata = file.metadata()?;
        if metadata.len() > (MAX_BATCH + 8192) as u64 {
            return Err(io::Error::other("spool record too large"));
        }
        if !metadata.is_file() {
            return Err(io::Error::other("spool entry is not a regular file"));
        }

        let mut bytes = Vec::new();
        file.take((MAX_BATCH + 8193) as u64).read_to_end(&mut bytes)?;
        if bytes.len() > MAX_BATCH + 8192 {
            return Err(io::Error::other("spool record too large"));
        }
        decode(&bytes)
    }

    /// Delete only after an acknowledged request, including rejected entries.
    /// A failed request keeps its file; deletion and directory sync can fail after ACK.
    fn finish_upload(&self, path: &Path, result: Result<usize, UploadError>) -> io::Result<bool> {
        match result {
            Ok(rejected) => {
                if rejected > 0 {
                    tracing::info!(rejected, "unsupported mail entries discarded");
                }
                fs::remove_file(path)?;
                File::open(&self.path)?.sync_all()?;
                Ok(true)
            }
            Err(error) => {
                tracing::warn!(%error, "upload retained for retry");
                Ok(false)
            }
        }
    }

    /// Upload committed files indefinitely, pausing five seconds between scans.
    ///
    /// The first failed upload or acknowledgement cleanup ends the current scan,
    /// leaving later files for the next pass. Invalid local records are logged
    /// and removed. Acknowledged ingress rejections complete delivery and are
    /// not retried. A batch may contain both accepted and rejected entries.
    ///
    /// This future performs synchronous filesystem work and is intended for the
    /// service's multithreaded runtime. Cancellation can leave an acknowledged
    /// file for retry; correctness relies on ingress deduplication, not exactly-once
    /// delivery. Keep only one uploader active per spool.
    pub async fn upload_loop(&self, uploader: rokbattles_gateway_protocol::MailUploader) {
        loop {
            match self.pending() {
                Ok(paths) => {
                    for path in paths {
                        match Self::read(&path) {
                            Ok(batch) => {
                                let result = uploader.upload_once(&batch).await;
                                let rejected = result.as_ref().ok().copied();
                                match self.finish_upload(&path, result) {
                                    Ok(true) => {
                                        tracing::info!(
                                            entries = batch.entries.len(),
                                            rejected,
                                            "relay batch acknowledged"
                                        );
                                    }
                                    Ok(false) => break,
                                    Err(error) => {
                                        tracing::warn!(%error, "could not remove acknowledged batch");
                                        break;
                                    }
                                }
                            }
                            Err(error) => {
                                tracing::warn!(%error, "corrupt local spool record discarded");
                                if let Err(error) = fs::remove_file(&path) {
                                    tracing::warn!(%error, "could not remove corrupt record");
                                    break;
                                }
                            }
                        }
                    }
                }
                Err(error) => tracing::warn!(%error, "could not read upload spool"),
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
}

/// Encode the private spool envelope; protobuf entry bytes remain unchanged.
/// Layout: ROKG1, presence flags (u8), player ID (i64), server ID (i32), entry
/// count (u32), then length-prefixed entries (u32). All numeric fields are big-endian.
fn encode(batch: &MailBatch) -> io::Result<Vec<u8>> {
    let payload_bytes: usize = batch.entries.iter().map(Bytes::len).sum();
    if batch.entries.is_empty() || batch.entries.len() > 512 || payload_bytes > MAX_BATCH {
        return Err(io::Error::other("invalid upload batch size"));
    }

    let mut output = b"ROKG1".to_vec();
    output.push(
        u8::from(batch.context.player_id.is_some())
            | (u8::from(batch.context.server_id.is_some()) << 1),
    );
    output.extend(batch.context.player_id.unwrap_or_default().to_be_bytes());
    output.extend(batch.context.server_id.unwrap_or_default().to_be_bytes());
    output.extend((batch.entries.len() as u32).to_be_bytes());
    for entry in &batch.entries {
        output.extend((entry.len() as u32).to_be_bytes());
        output.extend(entry);
    }
    Ok(output)
}
/// Decode one complete envelope, rejecting incomplete or trailing data.
/// Presence flags distinguish an absent identity from an explicitly zero identity.
fn decode(mut input: &[u8]) -> io::Result<MailBatch> {
    fn take<const N: usize>(input: &mut &[u8]) -> io::Result<[u8; N]> {
        let value = input
            .get(..N)
            .and_then(|v| v.try_into().ok())
            .ok_or_else(|| io::Error::other("truncated spool record"))?;
        *input = input.get(N..).ok_or_else(|| io::Error::other("truncated spool record"))?;
        Ok(value)
    }

    if &take::<5>(&mut input)? != b"ROKG1" {
        return Err(io::Error::other("invalid spool version"));
    }
    let [flags] = take::<1>(&mut input)?;
    let player_id = i64::from_be_bytes(take(&mut input)?);
    let server_id = i32::from_be_bytes(take(&mut input)?);
    let count = u32::from_be_bytes(take(&mut input)?);
    if count == 0 || count > 512 || flags > 3 {
        return Err(io::Error::other("invalid spool header"));
    }

    let mut entries = Vec::new();
    let mut total = 0;
    for _ in 0..count {
        let length = u32::from_be_bytes(take(&mut input)?) as usize;
        total += length;
        if total > MAX_BATCH {
            return Err(io::Error::other("spool payload too large"));
        }
        let entry = input.get(..length).ok_or_else(|| io::Error::other("truncated mail entry"))?;
        entries.push(Bytes::copy_from_slice(entry));
        input = input.get(length..).ok_or_else(|| io::Error::other("truncated mail entry"))?;
    }
    if !input.is_empty() {
        return Err(io::Error::other("trailing spool bytes"));
    }
    Ok(MailBatch {
        context: MailContext {
            player_id: (flags & 1 != 0).then_some(player_id),
            server_id: (flags & 2 != 0).then_some(server_id),
        },
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobile_single_mail_burst_is_not_discarded_before_disk() {
        let directory = tempfile::tempdir().expect("directory");
        let spool = Spool::open(directory.path().to_path_buf()).expect("spool");
        // Deliberately stall the disk writer for the entire burst.
        let (writer, receiver) =
            Writer::channel(spool.clone(), MAX_QUEUED_BATCHES, MAX_QUEUED_BYTES);
        for serial in 0_u32..4096 {
            writer
                .try_send(MailBatch {
                    context: MailContext::default(),
                    entries: vec![Bytes::copy_from_slice(&serial.to_be_bytes())],
                })
                .expect("every mail must reach memory or durable storage");
        }
        let accounting = writer.queued_bytes.clone();
        assert!(accounting.load(Ordering::Relaxed) > 0);
        drop(writer);
        spool.write_loop(receiver);
        assert_eq!(accounting.load(Ordering::Relaxed), 0);
        let paths = spool.pending().expect("pending");
        assert_eq!(paths.len(), 8, "small batches should share durable files");
        let entries: Vec<_> =
            paths.iter().flat_map(|path| Spool::read(path).expect("batch").entries).collect();
        let expected: Vec<_> =
            (0_u32..4096).map(|serial| Bytes::copy_from_slice(&serial.to_be_bytes())).collect();
        assert_eq!(entries, expected);
    }

    #[test]
    fn byte_budget_saturation_commits_overflow_and_releases_reservations() {
        let directory = tempfile::tempdir().expect("directory");
        let spool = Spool::open(directory.path().to_path_buf()).expect("spool");
        let (writer, receiver) = Writer::channel(spool.clone(), 8, 512);
        for value in [1, 2] {
            writer
                .try_send(MailBatch {
                    context: MailContext::default(),
                    entries: vec![Bytes::from(vec![value; 256])],
                })
                .expect("queue or spool");
        }
        assert!(writer.queued_bytes.load(Ordering::Relaxed) <= 512);
        let paths = spool.pending().expect("overflow");
        assert_eq!(paths.len(), 1);
        assert_eq!(Spool::read(&paths[0]).expect("batch").entries[0], vec![2; 256]);
        assert_eq!(receiver.recv().expect("queued").into_batch().entries[0], vec![1; 256]);
        assert_eq!(writer.queued_bytes.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn disconnected_writer_commits_to_disk_without_leaking_memory_budget() {
        let directory = tempfile::tempdir().expect("directory");
        let spool = Spool::open(directory.path().to_path_buf()).expect("spool");
        let (writer, receiver) = Writer::channel(spool.clone(), 1, 512);
        drop(receiver);
        writer
            .try_send(MailBatch {
                context: MailContext::default(),
                entries: vec![Bytes::from_static(b"mail")],
            })
            .expect("fallback");
        assert_eq!(spool.pending().expect("pending").len(), 1);
        assert_eq!(writer.queued_bytes.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn coalescing_preserves_player_context_and_payload_limit() {
        let directory = tempfile::tempdir().expect("directory");
        let spool = Spool::open(directory.path().to_path_buf()).expect("spool");
        let (writer, receiver) = Writer::channel(spool.clone(), 8, MAX_QUEUED_BYTES);
        for (player_id, value, size) in
            [(1, 1, 1), (2, 2, MAX_BATCH / 2 + 1), (2, 3, MAX_BATCH / 2 + 1)]
        {
            writer
                .try_send(MailBatch {
                    context: MailContext { player_id: Some(player_id), server_id: Some(42) },
                    entries: vec![Bytes::from(vec![value; size])],
                })
                .expect("queued");
        }
        drop(writer);
        spool.write_loop(receiver);
        let paths = spool.pending().expect("pending");
        assert_eq!(paths.len(), 3);
        for (path, player_id) in paths.iter().zip([1, 2, 2]) {
            let batch = Spool::read(path).expect("batch");
            assert_eq!(
                batch.context,
                MailContext { player_id: Some(player_id), server_id: Some(42) }
            );
            assert_eq!(batch.entries.len(), 1);
        }
    }

    #[test]
    fn committed_batches_survive_restart_with_exact_context_and_payload() {
        let directory = tempfile::tempdir().expect("directory");
        let spool = Spool::open(directory.path().to_path_buf()).expect("spool");
        spool
            .submit(&MailBatch {
                context: MailContext { player_id: Some(42), server_id: Some(1804) },
                entries: vec![Bytes::from_static(b"mail\0bytes")],
            })
            .expect("commit");
        drop(spool);
        let recovered = Spool::open(directory.path().to_path_buf()).expect("reopen");
        let paths = recovered.pending().expect("pending");
        let batch = Spool::read(paths.first().expect("batch")).expect("decode");
        assert_eq!(
            (batch.context, batch.entries),
            (
                MailContext { player_id: Some(42), server_id: Some(1804) },
                vec![Bytes::from_static(b"mail\0bytes")]
            )
        );
    }

    #[test]
    fn full_disk_budget_recovers_after_space_is_freed() {
        let directory = tempfile::tempdir().expect("directory");
        let spool = Spool::open(directory.path().to_path_buf()).expect("spool");
        let occupied = directory.path().join("old.batch");
        File::create(&occupied).expect("sparse file").set_len(MAX_SPOOL).expect("budget");
        let batch = MailBatch {
            context: MailContext::default(),
            entries: vec![Bytes::from_static(b"mail")],
        };
        spool.submit(&batch).expect_err("full budget must reject new batch");
        fs::remove_file(occupied).expect("free space");
        spool.submit(&batch).expect("later batches can commit");
        assert_eq!(spool.pending().expect("pending").len(), 1);
    }

    #[test]
    fn rejected_mail_is_deleted_but_failed_requests_are_retained() {
        let directory = tempfile::tempdir().expect("directory");
        let spool = Spool::open(directory.path().to_path_buf()).expect("spool");
        let batch = MailBatch {
            context: MailContext::default(),
            entries: vec![Bytes::from_static(b"mail")],
        };
        spool.submit(&batch).expect("first");
        spool.submit(&batch).expect("second");
        let paths = spool.pending().expect("pending");
        let first = paths.first().expect("first path");
        assert!(
            !spool.finish_upload(first, Err(UploadError::Acknowledgement)).expect("retain failure")
        );
        assert_eq!(spool.pending().expect("pending").len(), 2);
        assert!(spool.finish_upload(first, Ok(1)).expect("discard rejected mail"));
        assert_eq!(spool.pending().expect("pending").len(), 1);
        assert!(!first.exists());
    }

    #[test]
    fn oversized_and_symlinked_spool_records_are_rejected() {
        let directory = tempfile::tempdir().expect("directory");
        let record = directory.path().join("large.batch");
        File::create(&record)
            .expect("sparse file")
            .set_len((MAX_BATCH + 8193) as u64)
            .expect("length");
        Spool::read(&record).expect_err("oversized file");
        let link = directory.path().join("link.batch");
        std::os::unix::fs::symlink(&record, &link).expect("symlink");
        Spool::read(&link).expect_err("symlink");
    }

    #[test]
    fn startup_removes_only_unfinished_records_and_preserves_committed_mail() {
        let directory = tempfile::tempdir().expect("directory");
        let spool = Spool::open(directory.path().to_path_buf()).expect("spool");
        spool
            .submit(&MailBatch {
                context: MailContext::default(),
                entries: vec![Bytes::from_static(b"committed")],
            })
            .expect("commit");
        fs::write(directory.path().join("interrupted.tmp"), b"partial envelope")
            .expect("unfinished write");
        fs::write(directory.path().join("unrelated"), b"keep").expect("unrelated file");
        drop(spool);

        let reopened = Spool::open(directory.path().to_path_buf()).expect("restart");
        assert!(!directory.path().join("interrupted.tmp").exists());
        assert!(directory.path().join("unrelated").exists());
        let paths = reopened.pending().expect("committed records");
        assert_eq!(paths.len(), 1);
        assert_eq!(
            Spool::read(&paths[0]).expect("mail after restart").entries[0],
            b"committed".as_slice()
        );
    }

    #[test]
    fn envelope_preserves_absent_zero_and_signed_identity_values() {
        for player_id in [None, Some(0), Some(i64::MIN), Some(i64::MAX)] {
            for server_id in [None, Some(0), Some(i32::MIN), Some(i32::MAX)] {
                let context = MailContext { player_id, server_id };
                let batch =
                    MailBatch { context, entries: vec![Bytes::from_static(b"\x00\xffmail")] };
                let decoded = decode(&encode(&batch).expect("encode")).expect("decode");
                assert_eq!(decoded.context, batch.context);
                assert_eq!(decoded.entries, batch.entries);
            }
        }
    }

    #[test]
    fn malformed_envelopes_and_every_truncated_prefix_are_rejected() {
        let encoded = encode(&MailBatch {
            context: MailContext::default(),
            entries: vec![Bytes::from_static(b"mail")],
        })
        .expect("encode");
        for length in 0..encoded.len() {
            assert!(decode(&encoded[..length]).is_err(), "prefix length={length}");
        }
        let mut trailing = encoded.clone();
        trailing.push(0);
        decode(&trailing).expect_err("trailing bytes");
        for (offset, value) in [(0, b'X'), (5, 4)] {
            let mut invalid = encoded.clone();
            invalid[offset] = value;
            decode(&invalid).expect_err("invalid envelope");
        }
        for count in [0_u32, 513] {
            let mut invalid = encoded.clone();
            invalid[18..22].copy_from_slice(&count.to_be_bytes());
            decode(&invalid).expect_err("invalid envelope");
        }
        let mut too_large = encoded;
        too_large[22..26].copy_from_slice(&u32::MAX.to_be_bytes());
        decode(&too_large).expect_err("oversized entry length");
    }

    #[test]
    fn invalid_batches_do_not_create_committed_or_temporary_files() {
        let directory = tempfile::tempdir().expect("directory");
        let spool = Spool::open(directory.path().to_path_buf()).expect("spool");
        for entries in [Vec::new(), vec![Bytes::from_static(b"mail"); 513]] {
            assert!(spool.submit(&MailBatch { context: MailContext::default(), entries }).is_err());
        }
        assert_eq!(fs::read_dir(directory.path()).expect("directory").count(), 0);
    }

    #[test]
    fn concurrent_commits_share_the_final_available_disk_budget() {
        let directory = tempfile::tempdir().expect("directory");
        let spool = Spool::open(directory.path().to_path_buf()).expect("spool");
        let batch = MailBatch {
            context: MailContext::default(),
            entries: vec![Bytes::from_static(b"mail")],
        };
        let record_bytes = u64::try_from(encode(&batch).expect("encode").len()).expect("size");
        File::create(directory.path().join("occupied.batch"))
            .expect("sparse record")
            .set_len(MAX_SPOOL - record_bytes)
            .expect("reserve disk budget");
        let barrier = std::sync::Barrier::new(2);
        let committed = std::thread::scope(|scope| {
            let first = scope.spawn(|| {
                barrier.wait();
                spool.clone().submit(&batch).is_ok()
            });
            let second = scope.spawn(|| {
                barrier.wait();
                spool.clone().submit(&batch).is_ok()
            });
            usize::from(first.join().expect("first writer"))
                + usize::from(second.join().expect("second writer"))
        });
        assert_eq!(committed, 1, "the final record must not be overbooked");
        assert_eq!(spool.pending().expect("pending").len(), 2);
    }

    #[test]
    fn saturated_writer_reports_disk_failure_and_releases_its_queue_reservation() {
        let directory = tempfile::tempdir().expect("directory");
        let spool = Spool::open(directory.path().to_path_buf()).expect("spool");
        File::create(directory.path().join("occupied.batch"))
            .expect("sparse file")
            .set_len(MAX_SPOOL)
            .expect("full budget");
        let (writer, _receiver) = Writer::channel(spool, 0, 512);
        let error = writer
            .try_send(MailBatch {
                context: MailContext::default(),
                entries: vec![Bytes::from_static(b"mail")],
            })
            .expect_err("failed fallback must be visible");
        assert_eq!(error.to_string(), "upload spool full");
        assert_eq!(writer.queued_bytes.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn spool_reader_rejects_directories_without_reading_them_as_records() {
        let directory = tempfile::tempdir().expect("directory");
        Spool::read(directory.path()).expect_err("directory is not a record");
    }
}
