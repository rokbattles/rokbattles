use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tcp_stream::TcpStreamBatch;

const TCP_STREAM_QUEUE_FILE_NAME: &str = "tcp-stream-upload-queue.json";
const TCP_STREAM_UPLOAD_QUEUE_MAX_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct TcpStreamUploadQueueStore {
    version: u32,
    items: Vec<QueuedTcpStreamBatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct QueuedTcpStreamBatch {
    pub(super) batch: TcpStreamBatch,
    attempts: u32,
    not_before_ms: Option<u128>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TcpStreamBatchKey {
    capture_id: String,
    batch_index: u64,
}

#[derive(Debug, Default)]
pub(super) struct TcpStreamUploadQueue {
    items: Vec<QueuedTcpStreamBatch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct EnqueueBatchReport {
    queued: bool,
    dropped_captures: usize,
    dropped_batches: usize,
}

impl EnqueueBatchReport {
    pub(super) fn queued(self) -> bool {
        self.queued
    }

    pub(super) fn dropped_captures(self) -> usize {
        self.dropped_captures
    }

    pub(super) fn dropped_batches(self) -> usize {
        self.dropped_batches
    }
}

impl TcpStreamUploadQueue {
    pub(super) fn new(store: TcpStreamUploadQueueStore) -> Self {
        Self::new_with_max_bytes(store, TCP_STREAM_UPLOAD_QUEUE_MAX_BYTES)
    }

    fn new_with_max_bytes(store: TcpStreamUploadQueueStore, max_bytes: usize) -> Self {
        let mut items = Vec::new();
        for item in store.items {
            if !items.iter().any(|existing: &QueuedTcpStreamBatch| {
                tcp_stream_batch_key(&existing.batch) == tcp_stream_batch_key(&item.batch)
            }) {
                items.push(item);
            }
        }
        let mut queue = Self { items };
        queue.drop_oldest_captures_to_fit(max_bytes);
        queue
    }

    pub(super) fn enqueue_batch(&mut self, batch: TcpStreamBatch) -> EnqueueBatchReport {
        self.enqueue_batch_with_max_bytes(batch, TCP_STREAM_UPLOAD_QUEUE_MAX_BYTES)
    }

    fn enqueue_batch_with_max_bytes(
        &mut self,
        batch: TcpStreamBatch,
        max_bytes: usize,
    ) -> EnqueueBatchReport {
        let key = tcp_stream_batch_key(&batch);
        if self.items.iter().any(|item| tcp_stream_batch_key(&item.batch) == key) {
            return EnqueueBatchReport { queued: true, dropped_captures: 0, dropped_batches: 0 };
        }
        self.items.push(QueuedTcpStreamBatch { batch, attempts: 0, not_before_ms: None });
        let (dropped_captures, dropped_batches) = self.drop_oldest_captures_to_fit(max_bytes);
        let queued = self.items.iter().any(|item| tcp_stream_batch_key(&item.batch) == key);
        EnqueueBatchReport { queued, dropped_captures, dropped_batches }
    }

    pub(super) fn has_capture(&self, capture_id: &str) -> bool {
        self.items.iter().any(|item| item.batch.capture_id == capture_id)
    }

    pub(super) fn next_ready(&self, now_ms: u128) -> Option<QueuedTcpStreamBatch> {
        self.items
            .iter()
            .find(|item| {
                item.not_before_ms.is_none_or(|not_before_ms| not_before_ms <= now_ms)
                    // Final batches make a capture processable, so replay each capture in order.
                    && !self.has_earlier_batch_for_capture(item)
            })
            .cloned()
    }

    pub(super) fn remove_batch(&mut self, batch: &TcpStreamBatch) {
        let key = tcp_stream_batch_key(batch);
        self.items.retain(|item| tcp_stream_batch_key(&item.batch) != key);
    }

    pub(super) fn mark_failed(&mut self, batch: &TcpStreamBatch, now_ms: u128) {
        let key = tcp_stream_batch_key(batch);
        let Some(item) =
            self.items.iter_mut().find(|item| tcp_stream_batch_key(&item.batch) == key)
        else {
            return;
        };
        item.attempts = item.attempts.saturating_add(1);
        item.not_before_ms =
            Some(now_ms.saturating_add(tcp_stream_upload_backoff(item.attempts).as_millis()));
    }

    pub(super) fn store(&self) -> TcpStreamUploadQueueStore {
        TcpStreamUploadQueueStore { version: 1, items: self.items.clone() }
    }

    fn drop_oldest_captures_to_fit(&mut self, max_bytes: usize) -> (usize, usize) {
        let mut dropped_captures = 0usize;
        let mut dropped_batches = 0usize;

        while !queue_items_fit(&self.items, max_bytes) {
            let Some(capture_id) = self.items.first().map(|item| item.batch.capture_id.clone())
            else {
                break;
            };

            let before = self.items.len();
            self.items.retain(|item| item.batch.capture_id != capture_id);

            let removed = before.saturating_sub(self.items.len());
            if removed == 0 {
                break;
            }
            dropped_captures = dropped_captures.saturating_add(1);
            dropped_batches = dropped_batches.saturating_add(removed);
        }

        (dropped_captures, dropped_batches)
    }

    fn has_earlier_batch_for_capture(&self, item: &QueuedTcpStreamBatch) -> bool {
        self.items.iter().any(|candidate| {
            candidate.batch.capture_id == item.batch.capture_id
                && candidate.batch.batch_index < item.batch.batch_index
        })
    }
}

pub(super) fn read_tcp_stream_upload_queue(
    app: &AppHandle,
) -> anyhow::Result<TcpStreamUploadQueueStore> {
    let path = tcp_stream_upload_queue_file(app)?;
    if !path.exists() {
        return Ok(TcpStreamUploadQueueStore::default());
    }

    let metadata = fs::metadata(&path).with_context(|| format!("Failed reading {path:?}"))?;
    if metadata.len() > TCP_STREAM_UPLOAD_QUEUE_MAX_BYTES as u64 {
        fs::remove_file(&path).with_context(|| {
            format!("Failed removing oversized TCP stream upload queue {path:?}")
        })?;
        return Ok(TcpStreamUploadQueueStore::default());
    }

    let data = fs::read(&path).with_context(|| format!("Failed reading {path:?}"))?;
    if data.is_empty() {
        return Ok(TcpStreamUploadQueueStore::default());
    }

    serde_json::from_slice(&data).with_context(|| format!("Invalid JSON in {path:?}"))
}

pub(super) fn write_tcp_stream_upload_queue(
    app: &AppHandle,
    store: &TcpStreamUploadQueueStore,
) -> anyhow::Result<()> {
    let path = tcp_stream_upload_queue_file(app)?;
    let json = serde_json::to_vec(store).context("Failed to serialize TCP stream upload queue")?;
    if json.len() > TCP_STREAM_UPLOAD_QUEUE_MAX_BYTES {
        bail!(
            "TCP stream upload queue is {} bytes, exceeding the {} byte limit",
            json.len(),
            TCP_STREAM_UPLOAD_QUEUE_MAX_BYTES
        );
    }
    atomic_write(&path, &json).with_context(|| format!("Failed writing {path:?}"))
}

fn queue_items_fit(items: &[QueuedTcpStreamBatch], max_bytes: usize) -> bool {
    serialized_queue_len(items).is_some_and(|len| len <= max_bytes)
}

fn serialized_queue_len(items: &[QueuedTcpStreamBatch]) -> Option<usize> {
    let store = TcpStreamUploadQueueStore { version: 1, items: items.to_vec() };
    serde_json::to_vec(&store).ok().map(|json| json.len())
}

fn tcp_stream_batch_key(batch: &TcpStreamBatch) -> TcpStreamBatchKey {
    TcpStreamBatchKey { capture_id: batch.capture_id.clone(), batch_index: batch.batch_index }
}

fn tcp_stream_upload_backoff(attempts: u32) -> Duration {
    let seconds = 2u64.saturating_pow(attempts.min(10));
    Duration::from_secs(seconds.clamp(2, 300))
}

fn tcp_stream_upload_queue_file(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app.path().app_config_dir().context("Could not resolve app config directory")?;
    fs::create_dir_all(&dir).context("Failed to create app config directory")?;
    Ok(dir.join(TCP_STREAM_QUEUE_FILE_NAME))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let tmp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp_path)
        .with_context(|| format!("Failed creating temp file {tmp_path:?}"))?;
    file.write_all(bytes).with_context(|| format!("Failed writing temp file {tmp_path:?}"))?;

    if let Err(error) = fs::rename(&tmp_path, path) {
        if error.kind() == io::ErrorKind::AlreadyExists {
            let _ = fs::remove_file(path);
            fs::rename(&tmp_path, path).with_context(|| format!("Failed replacing {path:?}"))?;
        } else {
            return Err(error).with_context(|| format!("Failed renaming {tmp_path:?} -> {path:?}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use tcp_stream::{
        CLIENT_PORT, Direction, Handshake, StreamId, TcpStreamBatch, TcpStreamFragmentUpload,
    };

    use super::*;

    const HANDSHAKE_BODY: &[u8] = &[
        0x08, 0xf2, 0x42, 0x12, 0x0c, 0x08, 0x97, 0xd9, 0xd0, 0xaa, 0x02, 0x10, 0xd8, 0xb3, 0x98,
        0xf1, 0x03,
    ];

    #[test]
    fn tcp_stream_upload_queue_dedupes_batches() {
        let batch = tcp_stream_batch(0, false);
        let mut queue = TcpStreamUploadQueue::default();

        queue.enqueue_batch(batch.clone());
        queue.enqueue_batch(batch);

        assert_eq!(queue.store().items.len(), 1);
    }

    #[test]
    fn tcp_stream_upload_queue_drops_oldest_capture_when_serialized_size_exceeds_quota() {
        let first = tcp_stream_batch_for_capture("capture-1", 0, true);
        let second = tcp_stream_batch_for_capture("capture-2", 0, true);
        let mut queue = TcpStreamUploadQueue::default();
        queue.enqueue_batch(first);
        let max_bytes = serialized_queue_len(&queue.store().items).expect("queue should serialize");

        let report = queue.enqueue_batch_with_max_bytes(second.clone(), max_bytes);

        assert!(report.queued());
        assert_eq!(report.dropped_captures(), 1);
        assert_eq!(queue.store().items[0].batch.capture_id, second.capture_id);
    }

    #[test]
    fn tcp_stream_upload_queue_drops_new_batch_when_it_cannot_fit_quota() {
        let batch = tcp_stream_batch(0, true);
        let item = QueuedTcpStreamBatch { batch: batch.clone(), attempts: 0, not_before_ms: None };
        let max_bytes = serialized_queue_len(&[item]).expect("queue should serialize") - 1;
        let mut queue = TcpStreamUploadQueue::default();

        let report = queue.enqueue_batch_with_max_bytes(batch, max_bytes);

        assert!(!report.queued());
        assert!(queue.store().items.is_empty());
    }

    #[test]
    fn tcp_stream_upload_queue_trims_loaded_store_to_quota() {
        let first = tcp_stream_batch_for_capture("capture-1", 0, true);
        let second = tcp_stream_batch_for_capture("capture-2", 0, true);
        let mut queue = TcpStreamUploadQueue::default();
        queue.enqueue_batch(first);
        queue.enqueue_batch(second.clone());
        let store = queue.store();
        let max_bytes = serialized_queue_len(&store.items[1..]).expect("queue should serialize");

        let queue = TcpStreamUploadQueue::new_with_max_bytes(store, max_bytes);

        assert_eq!(queue.store().items.len(), 1);
        assert_eq!(queue.store().items[0].batch.capture_id, second.capture_id);
    }

    #[test]
    fn tcp_stream_upload_queue_preserves_final_batch_marker() {
        let batch = tcp_stream_batch(0, true);
        let mut queue = TcpStreamUploadQueue::default();

        queue.enqueue_batch(batch);
        let ready = queue.next_ready(0).expect("queued final batch should be ready");

        assert!(ready.batch.stream_ended);
    }

    #[test]
    fn tcp_stream_upload_queue_marks_failed_batches_for_later_retry() {
        let batch = tcp_stream_batch(0, true);
        let mut queue = TcpStreamUploadQueue::default();
        queue.enqueue_batch(batch.clone());

        queue.mark_failed(&batch, 1_000);

        assert!(queue.next_ready(1_001).is_none());
    }

    #[test]
    fn tcp_stream_upload_queue_removes_successful_batches() {
        let batch = tcp_stream_batch(0, true);
        let mut queue = TcpStreamUploadQueue::default();
        queue.enqueue_batch(batch.clone());

        queue.remove_batch(&batch);

        assert!(queue.next_ready(u128::MAX).is_none());
    }

    #[test]
    fn tcp_stream_upload_queue_blocks_later_capture_batches_until_earlier_batch_replays() {
        let first = tcp_stream_batch(0, false);
        let final_batch = tcp_stream_batch(1, true);
        let mut queue = TcpStreamUploadQueue::default();
        queue.enqueue_batch(first.clone());
        queue.mark_failed(&first, 1_000);
        queue.enqueue_batch(final_batch);

        assert!(queue.next_ready(1_001).is_none());
    }

    #[test]
    fn tcp_stream_upload_queue_allows_other_captures_when_one_capture_is_backed_off() {
        let first = tcp_stream_batch(0, false);
        let other = tcp_stream_batch_for_capture("capture-2", 0, true);
        let mut queue = TcpStreamUploadQueue::default();
        queue.enqueue_batch(first.clone());
        queue.mark_failed(&first, 1_000);
        queue.enqueue_batch(other.clone());

        let ready = queue.next_ready(1_001).expect("other capture should be ready");

        assert_eq!(ready.batch.capture_id, other.capture_id);
    }

    #[test]
    fn tcp_stream_upload_queue_tracks_pending_captures() {
        let batch = tcp_stream_batch(0, false);
        let mut queue = TcpStreamUploadQueue::default();

        queue.enqueue_batch(batch);

        assert!(queue.has_capture("capture-1"));
    }

    fn stream_id() -> StreamId {
        StreamId {
            client_addr: IpAddr::from(Ipv4Addr::new(10, 0, 0, 1)),
            client_port: 56_380,
            server_addr: IpAddr::from(Ipv4Addr::new(10, 0, 0, 2)),
            server_port: CLIENT_PORT,
        }
    }

    fn tcp_stream_batch(batch_index: u64, stream_ended: bool) -> TcpStreamBatch {
        tcp_stream_batch_for_capture("capture-1", batch_index, stream_ended)
    }

    fn tcp_stream_batch_for_capture(
        capture_id: &str,
        batch_index: u64,
        stream_ended: bool,
    ) -> TcpStreamBatch {
        TcpStreamBatch {
            capture_id: capture_id.to_string(),
            batch_index,
            stream_ended,
            stream: stream_id(),
            handshake: Handshake { api_id: 8562, key1: 1, key2: 2 },
            fragments: vec![TcpStreamFragmentUpload::from_payload(
                0,
                Direction::ServerToClient,
                &prefixed(HANDSHAKE_BODY),
            )],
        }
    }

    fn prefixed(body: &[u8]) -> Vec<u8> {
        let mut payload = Vec::from(u16::try_from(body.len()).unwrap().to_be_bytes());
        payload.extend_from_slice(body);
        payload
    }
}
