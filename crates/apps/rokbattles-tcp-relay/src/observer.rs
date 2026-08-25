//! Bounded, fail-open observation of copied server-stream bytes.

use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
};

use bytes::Bytes;
use tokio::{
    sync::{mpsc, oneshot},
    task::{JoinError, JoinHandle},
};
use tracing::warn;

use crate::{
    MailUploader, RuntimeArtifact,
    stream::{ServerStreamProcessor, StreamError, StreamEvent},
    uploader::{MailBatch, MailContext},
};

const OBSERVER_QUEUE_CHUNKS: usize = 512;
const UPLOAD_QUEUE_BATCHES: usize = 32;
const MAX_UPLOAD_BATCH_ENTRIES: usize = 512;
const MAX_UPLOAD_BATCH_BYTES: usize = 24 * 1024 * 1024;
const ACTIVE: u8 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DisableReason {
    QueueFull = 1,
    ObserverStopped = 2,
    UnsupportedHandshake = 3,
    MalformedFrame = 4,
    CipherState = 5,
    Decompression = 6,
    CipherOrSchemaDrift = 7,
    ObserverTaskFailed = 8,
    UploadQueueFull = 9,
    UploadWorkerStopped = 10,
    UploadEntryTooLarge = 11,
}

impl DisableReason {
    #[cfg(test)]
    pub(crate) const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::QueueFull),
            2 => Some(Self::ObserverStopped),
            3 => Some(Self::UnsupportedHandshake),
            4 => Some(Self::MalformedFrame),
            5 => Some(Self::CipherState),
            6 => Some(Self::Decompression),
            7 => Some(Self::CipherOrSchemaDrift),
            8 => Some(Self::ObserverTaskFailed),
            9 => Some(Self::UploadQueueFull),
            10 => Some(Self::UploadWorkerStopped),
            11 => Some(Self::UploadEntryTooLarge),
            _ => None,
        }
    }

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::QueueFull => "queue_full",
            Self::ObserverStopped => "observer_stopped",
            Self::UnsupportedHandshake => "unsupported_handshake",
            Self::MalformedFrame => "malformed_frame",
            Self::CipherState => "cipher_state",
            Self::Decompression => "decompression",
            Self::CipherOrSchemaDrift => "cipher_or_schema_drift",
            Self::ObserverTaskFailed => "observer_task_failed",
            Self::UploadQueueFull => "upload_queue_full",
            Self::UploadWorkerStopped => "upload_worker_stopped",
            Self::UploadEntryTooLarge => "upload_entry_too_large",
        }
    }
}

impl From<StreamError> for DisableReason {
    fn from(value: StreamError) -> Self {
        match value {
            StreamError::FrameTooLarge => Self::MalformedFrame,
            StreamError::UnsupportedHandshake => Self::UnsupportedHandshake,
            StreamError::CipherUnavailable => Self::CipherState,
            StreamError::Decompression => Self::Decompression,
            StreamError::Protocol => Self::CipherOrSchemaDrift,
        }
    }
}

#[derive(Debug)]
pub(crate) struct StreamObserver {
    sender: Option<mpsc::Sender<Vec<u8>>>,
    state: Arc<AtomicU8>,
    task: JoinHandle<()>,
    upload_task: Option<JoinHandle<()>>,
    client_addr: SocketAddr,
}

impl StreamObserver {
    pub(crate) fn spawn(
        artifact: Arc<RuntimeArtifact>,
        uploader: Option<MailUploader>,
        client_addr: SocketAddr,
    ) -> Self {
        Self::spawn_inner(artifact, uploader, client_addr, OBSERVER_QUEUE_CHUNKS, None)
    }

    fn spawn_inner(
        artifact: Arc<RuntimeArtifact>,
        uploader: Option<MailUploader>,
        client_addr: SocketAddr,
        capacity: usize,
        start: Option<oneshot::Receiver<()>>,
    ) -> Self {
        let (sender, mut receiver) = mpsc::channel::<Vec<u8>>(capacity);
        let (upload_sender, upload_task) = uploader.map_or((None, None), |uploader| {
            let (sender, mut receiver) = mpsc::channel::<MailBatch>(UPLOAD_QUEUE_BATCHES);
            let task = tokio::spawn(async move {
                while let Some(batch) = receiver.recv().await {
                    uploader.upload(batch).await;
                }
            });
            (Some(sender), Some(task))
        });
        let state = Arc::new(AtomicU8::new(ACTIVE));
        let task_state = Arc::clone(&state);
        let task = tokio::spawn(async move {
            if let Some(start) = start {
                let _start_result = start.await;
            }
            let mut processor = ServerStreamProcessor::new(&artifact);
            let mut context = MailContext::default();
            let mut pending = MailBatchAccumulator::default();
            while task_state.load(Ordering::Acquire) == ACTIVE {
                let Some(bytes) = receiver.recv().await else {
                    break;
                };
                if task_state.load(Ordering::Acquire) != ACTIVE {
                    break;
                }
                match processor.push(&bytes) {
                    Ok(events) => {
                        for event in events {
                            match event {
                                StreamEvent::Login { player_id, server_id } => {
                                    context = MailContext {
                                        player_id: Some(player_id),
                                        server_id: Some(server_id),
                                    };
                                }
                                StreamEvent::Mails { server_id, entries, remaining } => {
                                    let Some(sender) = &upload_sender else {
                                        continue;
                                    };
                                    learn_server_id(&mut context, server_id);
                                    if let Err(reason) =
                                        pending.push(sender, &context, entries, remaining)
                                    {
                                        disable_once(&task_state, client_addr, reason);
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    Err(error) => {
                        disable_once(&task_state, client_addr, error.into());
                        break;
                    }
                }
            }
            if let Some(sender) = &upload_sender
                && let Err(reason) = pending.flush(sender, &context)
            {
                disable_once(&task_state, client_addr, reason);
            }
        });
        Self { sender: Some(sender), state, task, upload_task, client_addr }
    }

    #[cfg(test)]
    pub(crate) fn spawn_paused(
        artifact: Arc<RuntimeArtifact>,
        client_addr: SocketAddr,
        capacity: usize,
    ) -> (Self, oneshot::Sender<()>) {
        let (start_sender, start_receiver) = oneshot::channel();
        (
            Self::spawn_inner(artifact, None, client_addr, capacity, Some(start_receiver)),
            start_sender,
        )
    }

    /// Offer copied bytes without waiting for processing capacity.
    pub(crate) fn observe(&mut self, bytes: &[u8]) {
        if self.state.load(Ordering::Acquire) != ACTIVE {
            self.sender.take();
            return;
        }
        let Some(sender) = &self.sender else {
            return;
        };
        match sender.try_send(bytes.to_vec()) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_bytes)) => {
                disable_once(&self.state, self.client_addr, DisableReason::QueueFull);
                self.sender.take();
            }
            Err(mpsc::error::TrySendError::Closed(_bytes)) => {
                disable_once(&self.state, self.client_addr, DisableReason::ObserverStopped);
                self.sender.take();
            }
        }
    }

    pub(crate) async fn finish(mut self) {
        self.sender.take();
        if let Err(error) = self.task.await {
            record_join_failure(&self.state, self.client_addr, &error);
        }
        if let Some(task) = self.upload_task
            && let Err(error) = task.await
        {
            record_join_failure(&self.state, self.client_addr, &error);
        }
    }
}

fn learn_server_id(context: &mut MailContext, candidate: Option<i32>) {
    if context.server_id.is_some() {
        return;
    }
    context.server_id = candidate.filter(|server_id| *server_id != 0);
}

#[derive(Debug, Default)]
struct MailBatchAccumulator {
    entries: Vec<Vec<u8>>,
    bytes: usize,
}

impl MailBatchAccumulator {
    fn push(
        &mut self,
        sender: &mpsc::Sender<MailBatch>,
        context: &MailContext,
        entries: Vec<Vec<u8>>,
        remaining: Option<usize>,
    ) -> Result<(), DisableReason> {
        for entry in entries {
            if entry.len() > MAX_UPLOAD_BATCH_BYTES {
                return Err(DisableReason::UploadEntryTooLarge);
            }
            if !self.entries.is_empty()
                && (self.entries.len() >= MAX_UPLOAD_BATCH_ENTRIES
                    || self.bytes.saturating_add(entry.len()) > MAX_UPLOAD_BATCH_BYTES)
            {
                self.flush(sender, context)?;
            }
            self.bytes = self.bytes.saturating_add(entry.len());
            self.entries.push(entry);
        }
        if remaining.is_none_or(|remaining| remaining == 0) {
            self.flush(sender, context)?;
        }
        Ok(())
    }

    fn flush(
        &mut self,
        sender: &mpsc::Sender<MailBatch>,
        context: &MailContext,
    ) -> Result<(), DisableReason> {
        if self.entries.is_empty() {
            return Ok(());
        }
        let entries = std::mem::take(&mut self.entries);
        try_submit(sender, context, entries)?;
        self.bytes = 0;
        Ok(())
    }
}

fn try_submit(
    sender: &mpsc::Sender<MailBatch>,
    context: &MailContext,
    entries: Vec<Vec<u8>>,
) -> Result<(), DisableReason> {
    let entries = entries.into_iter().map(Bytes::from).collect();
    let batch = MailBatch { context: context.clone(), entries };
    sender.try_send(batch).map_err(|error| match error {
        mpsc::error::TrySendError::Full(_batch) => DisableReason::UploadQueueFull,
        mpsc::error::TrySendError::Closed(_batch) => DisableReason::UploadWorkerStopped,
    })
}

fn disable_once(state: &AtomicU8, client_addr: SocketAddr, reason: DisableReason) {
    if state.compare_exchange(ACTIVE, reason as u8, Ordering::AcqRel, Ordering::Acquire).is_ok() {
        warn!(
            %client_addr,
            reason = reason.as_str(),
            "TCP stream observer stopped capturing; byte forwarding remains active"
        );
    }
}

fn record_join_failure(state: &AtomicU8, client_addr: SocketAddr, error: &JoinError) {
    let reason = DisableReason::ObserverTaskFailed;
    if state.compare_exchange(ACTIVE, reason as u8, Ordering::AcqRel, Ordering::Acquire).is_ok() {
        warn!(
            %client_addr,
            reason = reason.as_str(),
            cancelled = error.is_cancelled(),
            panicked = error.is_panic(),
            "TCP stream observer stopped capturing; byte forwarding was unaffected"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mail_server_id_initializes_missing_connection_context() {
        let mut context = MailContext::default();

        learn_server_id(&mut context, Some(16_012));

        assert_eq!(context.server_id, Some(16_012));
    }

    #[test]
    fn mail_server_id_does_not_replace_login_context() {
        let mut context = MailContext { player_id: Some(123), server_id: Some(1_804) };

        learn_server_id(&mut context, Some(16_012));

        assert_eq!(context.server_id, Some(1_804));
    }

    #[test]
    fn zero_mail_server_id_does_not_initialize_connection_context() {
        let mut context = MailContext::default();

        learn_server_id(&mut context, Some(0));

        assert_eq!(context.server_id, None);
    }

    #[tokio::test]
    async fn unsupported_handshake_disables_only_the_observer() {
        let artifact = Arc::new(RuntimeArtifact::test_fixture());
        let client_addr = "127.0.0.1:12345".parse().expect("address should parse");
        let mut observer = StreamObserver::spawn(artifact, None, client_addr);

        observer.observe(&[0x00, 0x02, 0x08, 0x01]);
        tokio::task::yield_now().await;
        observer.observe(b"bytes ignored after disablement");
        let state = Arc::clone(&observer.state);
        observer.finish().await;

        assert_eq!(
            DisableReason::from_code(state.load(Ordering::Acquire)),
            Some(DisableReason::UnsupportedHandshake)
        );
    }

    #[tokio::test]
    async fn full_queue_permanently_disables_observation_without_waiting() {
        let artifact = Arc::new(RuntimeArtifact::test_fixture());
        let client_addr = "127.0.0.1:12345".parse().expect("address should parse");
        let (mut observer, start_sender) = StreamObserver::spawn_paused(artifact, client_addr, 1);

        observer.observe(b"fills the only queue slot");
        observer.observe(b"must be dropped immediately");
        let _send_result = start_sender.send(());
        let state = Arc::clone(&observer.state);
        observer.finish().await;

        assert_eq!(
            DisableReason::from_code(state.load(Ordering::Acquire)),
            Some(DisableReason::QueueFull)
        );
    }

    #[tokio::test]
    async fn mail_entries_are_split_at_the_ingress_batch_count() {
        let (sender, mut receiver) = mpsc::channel(2);
        let context = MailContext { player_id: Some(123), server_id: Some(1804) };
        let entries = (0..MAX_UPLOAD_BATCH_ENTRIES + 1).map(|_| vec![1]).collect();
        let mut pending = MailBatchAccumulator::default();

        pending.push(&sender, &context, entries, Some(0)).expect("batches should enqueue");
        let first = receiver.recv().await.expect("first batch should exist");
        let second = receiver.recv().await.expect("second batch should exist");

        assert_eq!(first.entries.len(), MAX_UPLOAD_BATCH_ENTRIES);
        assert_eq!(second.entries.len(), 1);
        assert_eq!(first.context, context);
        assert_eq!(second.context, context);
    }

    #[tokio::test]
    async fn paginated_initial_sync_is_coalesced_into_bounded_batches() {
        let (sender, mut receiver) = mpsc::channel(UPLOAD_QUEUE_BATCHES);
        let context = MailContext::default();
        let mut pending = MailBatchAccumulator::default();
        let mut remaining = 974;

        while remaining > 0 {
            let entry_count = remaining.min(30);
            remaining -= entry_count;
            pending
                .push(&sender, &context, vec![vec![1]; entry_count], Some(remaining))
                .expect("initial mail sync should fit in the upload queue");
        }
        drop(sender);

        let mut batch_sizes = Vec::new();
        while let Some(batch) = receiver.recv().await {
            batch_sizes.push(batch.entries.len());
        }

        assert_eq!(batch_sizes, [512, 462]);
    }

    #[test]
    fn saturated_upload_queue_is_reported_without_waiting() {
        let (sender, _receiver) = mpsc::channel(1);
        let entries = (0..MAX_UPLOAD_BATCH_ENTRIES + 1).map(|_| vec![1]).collect();
        let mut pending = MailBatchAccumulator::default();

        let error = pending
            .push(&sender, &MailContext::default(), entries, Some(0))
            .expect_err("second batch should exceed queue capacity");

        assert_eq!(error, DisableReason::UploadQueueFull);
    }
}
