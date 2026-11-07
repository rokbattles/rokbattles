//! Reads and classifies source files independently of network uploads.
//!
//! Batches limit file count and retained bytes. A batch may exceed its byte budget
//! by one mail, bounded by [`super::store::MAX_MAIL_BYTES`], so large reports still
//! make progress. The runtime commits successful reads before starting uploads.

use tokio::sync::watch;

use super::{AttemptOutcome, now_epoch_ms, read_supported_mail, store::QueuedUpload};

pub(super) const READER_COUNT: usize = 4;
pub(super) const BATCH_FILES: usize = 64;
const BATCH_BYTES: usize = 4 * 1024 * 1024;

pub(super) struct PreparedSource {
    pub(super) item: QueuedUpload,
    pub(super) result: Result<Vec<u8>, AttemptOutcome>,
}

pub(super) struct PreparedBatch {
    pub(super) sources: Vec<PreparedSource>,
    /// Unread sources whose reservations must be requeued.
    pub(super) deferred: Vec<QueuedUpload>,
}

/// Reads a batch until its file or byte budget is reached, or shutdown is requested.
///
/// Checks shutdown between files. The caller must await this future during
/// shutdown because a running blocking read cannot be interrupted.
pub(super) async fn prepare_batch(
    items: Vec<QueuedUpload>,
    shutdown: watch::Receiver<bool>,
) -> anyhow::Result<PreparedBatch> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut sources = Vec::new();
        let mut retained_bytes = 0;
        let mut items = items.into_iter();

        while sources.len() < BATCH_FILES && retained_bytes < BATCH_BYTES && !*shutdown.borrow() {
            let Some(mut item) = items.next() else {
                break;
            };
            let result = read_supported_mail(&item, now_epoch_ms()).map(|prepared| {
                item.sig = Some(prepared.sig);
                item.attempts = 0;
                item.not_before_ms = None;
                retained_bytes += prepared.bytes.len();

                prepared.bytes
            });
            sources.push(PreparedSource { item, result });
        }

        PreparedBatch { sources, deferred: items.collect() }
    })
    .await
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use serde_json::json;

    use super::*;
    use crate::watcher::{
        SupportedMail,
        store::{MAX_MAIL_BYTES, file_sig},
    };

    fn write_source(dir: &Path, id: u32, value: &serde_json::Value) -> QueuedUpload {
        let path = dir.join(format!("Persistent.Mail.{id}"));
        fs::write(&path, rokbattles_mail_codec::encode(value).expect("encoded mail"))
            .expect("source");
        QueuedUpload {
            path: path.to_string_lossy().into_owned(),
            sig: None,
            attempts: 0,
            not_before_ms: None,
        }
    }

    #[test]
    fn complete_fresh_mail_is_captured_without_an_age_or_discovery_signature_delay() {
        let dir = tempfile::tempdir().expect("source directory");
        let item = write_source(dir.path(), 1, &json!({"id":"1", "type":"Battle"}));
        let sig = file_sig(&fs::metadata(&item.path).expect("metadata")).expect("signature");
        let Ok(SupportedMail { bytes, sig: captured_sig }) =
            read_supported_mail(&item, sig.modified + 1)
        else {
            panic!("a complete checksummed file should not wait for its timestamp to age");
        };

        assert_eq!(bytes, fs::read(&item.path).expect("original bytes"));
        assert_eq!(captured_sig, sig);
    }

    #[test]
    fn partial_fresh_mail_waits_and_is_captured_when_completed() {
        let dir = tempfile::tempdir().expect("source directory");
        let item = write_source(dir.path(), 1, &json!({"id":"1", "type":"Battle"}));
        let bytes = fs::read(&item.path).expect("original bytes");
        fs::write(&item.path, bytes.get(..10).expect("partial header")).expect("partial source");
        let sig = file_sig(&fs::metadata(&item.path).expect("metadata")).expect("signature");

        assert!(matches!(
            read_supported_mail(&item, sig.modified + 1),
            Err(AttemptOutcome::Waiting(_))
        ));
        fs::write(&item.path, &bytes).expect("complete source");
        let sig = file_sig(&fs::metadata(&item.path).expect("metadata")).expect("signature");
        assert!(read_supported_mail(&item, sig.modified + 1).is_ok());
    }

    #[test]
    fn oversized_source_is_rejected_before_reading_or_allocating_its_contents() {
        let dir = tempfile::tempdir().expect("source directory");
        let item = write_source(dir.path(), 1, &json!({"id":"1", "type":"Battle"}));
        fs::File::options()
            .write(true)
            .open(&item.path)
            .expect("source")
            .set_len(MAX_MAIL_BYTES + 1)
            .expect("large source");

        assert!(
            matches!(read_supported_mail(&item, now_epoch_ms()), Err(AttemptOutcome::Rejected(reason)) if reason.contains("oversized"))
        );
    }

    #[tokio::test]
    async fn byte_budget_defers_remaining_sources_and_limits_preparation_memory() {
        let dir = tempfile::tempdir().expect("source directory");
        let value = json!({"id":"1", "type":"Battle", "body":"x".repeat(BATCH_BYTES / 2 + 1)});
        let items = (1..=3).map(|id| write_source(dir.path(), id, &value)).collect();
        let (_stop, shutdown) = watch::channel(false);
        let batch = prepare_batch(items, shutdown).await.expect("read batch");

        assert_eq!(batch.sources.len(), 2);
        assert!(batch.sources.iter().all(|source| source.result.is_ok()));
        assert_eq!(batch.deferred.len(), 1);
        assert!(
            batch.deferred.iter().all(|item| item.sig.is_none()),
            "unused sources were not opened"
        );
    }

    #[tokio::test]
    async fn shutdown_defers_sources_that_have_not_started_reading() {
        let dir = tempfile::tempdir().expect("source directory");
        let item = write_source(dir.path(), 1, &json!({"id":"1", "type":"Battle"}));
        let (_stop, shutdown) = watch::channel(true);
        let batch = prepare_batch(vec![item.clone()], shutdown).await.expect("stopped batch");

        assert!(batch.sources.is_empty());
        assert_eq!(batch.deferred.first().expect("deferred source").path, item.path);
    }

    #[tokio::test]
    async fn file_budget_bounds_batches_even_when_every_source_is_unsupported() {
        let dir = tempfile::tempdir().expect("source directory");
        let value = json!({"id":"1", "type":"Unsupported"});
        let items = (0..=BATCH_FILES)
            .map(|id| write_source(dir.path(), u32::try_from(id).expect("fixture ID"), &value))
            .collect();
        let (_stop, shutdown) = watch::channel(false);
        let batch = prepare_batch(items, shutdown).await.expect("unsupported batch");

        assert_eq!(batch.sources.len(), BATCH_FILES);
        assert!(
            batch
                .sources
                .iter()
                .all(|source| matches!(source.result, Err(AttemptOutcome::Unsupported { .. })))
        );
        assert_eq!(batch.deferred.len(), 1);
    }
}
