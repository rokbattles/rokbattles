//! Bounded, fail-open observation of copied server-stream bytes.

use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
};

use tokio::{
    sync::{mpsc, oneshot},
    task::{JoinError, JoinHandle},
};
use tracing::warn;

use crate::{
    RuntimeArtifact,
    stream::{ServerStreamProcessor, StreamError},
};

const OBSERVER_QUEUE_CHUNKS: usize = 512;
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
    client_addr: SocketAddr,
}

impl StreamObserver {
    pub(crate) fn spawn(artifact: Arc<RuntimeArtifact>, client_addr: SocketAddr) -> Self {
        Self::spawn_inner(artifact, client_addr, OBSERVER_QUEUE_CHUNKS, None)
    }

    fn spawn_inner(
        artifact: Arc<RuntimeArtifact>,
        client_addr: SocketAddr,
        capacity: usize,
        start: Option<oneshot::Receiver<()>>,
    ) -> Self {
        let (sender, mut receiver) = mpsc::channel::<Vec<u8>>(capacity);
        let state = Arc::new(AtomicU8::new(ACTIVE));
        let task_state = Arc::clone(&state);
        let task = tokio::spawn(async move {
            if let Some(start) = start {
                let _start_result = start.await;
            }
            let mut processor = ServerStreamProcessor::new(&artifact);
            while task_state.load(Ordering::Acquire) == ACTIVE {
                let Some(bytes) = receiver.recv().await else {
                    break;
                };
                if task_state.load(Ordering::Acquire) != ACTIVE {
                    break;
                }
                if let Err(error) = processor.push(&bytes) {
                    disable_once(&task_state, client_addr, error.into());
                    break;
                }
            }
        });
        Self { sender: Some(sender), state, task, client_addr }
    }

    #[cfg(test)]
    pub(crate) fn spawn_paused(
        artifact: Arc<RuntimeArtifact>,
        client_addr: SocketAddr,
        capacity: usize,
    ) -> (Self, oneshot::Sender<()>) {
        let (start_sender, start_receiver) = oneshot::channel();
        (Self::spawn_inner(artifact, client_addr, capacity, Some(start_receiver)), start_sender)
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
    }
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

    #[tokio::test]
    async fn unsupported_handshake_disables_only_the_observer() {
        let artifact = Arc::new(RuntimeArtifact::test_fixture());
        let client_addr = "127.0.0.1:12345".parse().expect("address should parse");
        let mut observer = StreamObserver::spawn(artifact, client_addr);

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
}
