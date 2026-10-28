//! Bounded durable mail delivery, independent of TCP decoder lifetime.
//!
//! Completed batches are atomically renamed before becoming upload candidates.
//! A full disk budget drops the new batch with an error; future batches remain
//! eligible after space is freed. No flow is disabled by an upload failure.

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

/// Buffer bursts without letting large batches exhaust the service's memory.
pub struct Writer {
    sender: SyncSender<QueuedBatch>,
    queued_bytes: Arc<AtomicUsize>,
    max_bytes: usize,
    spool: Spool,
}

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
        self.queued_bytes.fetch_sub(self.bytes, Ordering::Relaxed);
    }
}

impl QueuedBatch {
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

    /// Queue without waiting in normal operation. At either memory limit, commit
    /// directly to disk rather than discard the batch. This last resort can delay
    /// passive capture, but never blocks kernel forwarding.
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

/// A directory owned exclusively by the unprivileged gateway account.
#[derive(Clone)]
pub struct Spool {
    path: PathBuf,
    commit_lock: Arc<Mutex<()>>,
}
impl Spool {
    /// Buffer up to 8192 batches and 128 MiB of payload and batch metadata.
    /// Coalesce queued batches for the same player before syncing to disk.
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

    fn write_loop(&self, receiver: Receiver<QueuedBatch>) {
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

    pub fn open(path: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&path)?;
        let spool = Self { path, commit_lock: Arc::new(Mutex::new(())) };
        // A crash before rename never produced a committed batch.
        for entry in fs::read_dir(&spool.path)? {
            let path = entry?.path();
            if path.extension().is_some_and(|ext| ext == "tmp") {
                fs::remove_file(path)?;
            }
        }
        Ok(spool)
    }

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

    pub fn submit(&self, batch: &MailBatch) -> io::Result<()> {
        let bytes = encode(batch)?;
        // The background writer and the saturation fallback share the disk budget.
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

    /// Read a complete, regular spool file without following symlinks.
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

    /// Remove acknowledged batches, including unsupported entries. Only failed
    /// requests remain queued; rejection is a completed ingress decision.
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

    /// Retry failed requests without blocking capture. Successfully processed
    /// batches are removed even when ingress reports unsupported mail.
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
}
