//! One ephemeral, bounded HTTP stream. Transport loss is a capture gap: callers
//! must discard every Observer flow and wait for fresh TCP handshakes, never
//! retry ciphertext against a new server-side cipher. Earlier mails may already
//! have stored before a failed response. No raw replay file/spool exists.

use std::{collections::BTreeMap, convert::Infallible, time::Duration};

use bytes::Bytes;
use rokbattles_capture_runtime::{lifecycle::Event, wire};
use serde::Deserialize;
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_stream::wrappers::ReceiverStream;
use zeroize::{Zeroize, Zeroizing};

const URL: &str = "https://ingress.rokbattles.com/v3/desktop/capture";
const QUEUE_FRAMES: usize = 128;
const MAX_FLOWS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Gap,
    Capacity,
    Sequence,
    Transport,
    Response,
}

#[derive(Debug, Deserialize)]
pub struct Report {
    pub stored: u64,
    pub skipped: u64,
    pub rejected: u64,
}

struct WipingBytes(Zeroizing<Vec<u8>>);

impl AsRef<[u8]> for WipingBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

struct WipingEvent(Event);

impl Drop for WipingEvent {
    fn drop(&mut self) {
        if let Event::Data { bytes, .. } = &mut self.0 {
            bytes.zeroize();
        }
    }
}

struct Outbox {
    sender: Option<mpsc::Sender<Result<Bytes, Infallible>>>,
    offsets: BTreeMap<u64, u64>,
    last_id: u64,
    failed: bool,
}

impl Outbox {
    fn enqueue(&mut self, event: Event) -> Result<(), Error> {
        let event = WipingEvent(event);
        if self.failed {
            return Err(Error::Gap);
        }
        let result = self.push(&event.0);
        if result.is_err() {
            self.poison();
        }
        result
    }

    fn push(&mut self, event: &Event) -> Result<(), Error> {
        match event {
            Event::Open { id, .. } => {
                if *id <= self.last_id {
                    return Err(Error::Sequence);
                }
                if self.offsets.len() >= MAX_FLOWS {
                    return Err(Error::Capacity);
                }
                self.offsets.insert(*id, 0);
                self.last_id = *id;
            }
            Event::Data { id, offset, bytes } => {
                let expected = self.offsets.get_mut(id).ok_or(Error::Sequence)?;
                if offset != expected {
                    return Err(Error::Sequence);
                }
                *expected = expected
                    .checked_add(u64::try_from(bytes.len()).map_err(|_error| Error::Capacity)?)
                    .ok_or(Error::Capacity)?;
            }
            Event::Close { id } | Event::Abort { id, .. } => {
                if self.offsets.remove(id).is_none() {
                    return Err(Error::Sequence);
                }
            }
            Event::Gap => return Err(Error::Gap),
            Event::Keepalive => {}
        }
        let sender = self.sender.as_ref().ok_or(Error::Gap)?;
        let frames: Vec<_> = wire::encode(event)
            .map_err(|_error| Error::Sequence)?
            .into_iter()
            .map(Zeroizing::new)
            .collect();
        for bytes in frames {
            let bytes = Bytes::from_owner(WipingBytes(bytes));
            sender.try_send(Ok(bytes)).map_err(|_error| Error::Capacity)?;
        }
        Ok(())
    }

    fn poison(&mut self) {
        self.failed = true;
        self.offsets.clear();
        self.sender.take();
    }
}

pub struct CaptureUpload {
    outbox: Outbox,
    task: Option<JoinHandle<Result<Report, Error>>>,
}

impl CaptureUpload {
    /// Only call after explicit capture consent and the first validated Open.
    /// This starts a real HTTPS request; synthetic tests exercise Outbox only.
    pub fn start() -> Result<Self, Error> {
        let client = reqwest::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .tcp_keepalive(Duration::from_secs(30))
            .user_agent(format!("ROKBattles/{} CaptureAgent", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_error| Error::Transport)?;
        let (sender, receiver) = mpsc::channel(QUEUE_FRAMES);
        let task = tokio::spawn(async move {
            let body = reqwest::Body::wrap_stream(ReceiverStream::new(receiver));
            let mut response = client
                .post(URL)
                .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
                .body(body)
                .send()
                .await
                .map_err(|_error| Error::Transport)?;
            if !response.status().is_success() {
                return Err(Error::Response);
            }
            if response.content_length().is_some_and(|size| size > 4096) {
                return Err(Error::Response);
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_error| Error::Transport)? {
                if bytes.len() + chunk.len() > 4096 {
                    return Err(Error::Response);
                }
                bytes.extend_from_slice(&chunk);
            }
            serde_json::from_slice(&bytes).map_err(|_error| Error::Response)
        });
        Ok(Self {
            outbox: Outbox {
                sender: Some(sender),
                offsets: BTreeMap::new(),
                last_id: 0,
                failed: false,
            },
            task: Some(task),
        })
    }

    pub fn event(&mut self, event: Event) -> Result<(), Error> {
        if self.ended() {
            self.outbox.poison();
            if let Event::Data { mut bytes, .. } = event {
                bytes.zeroize();
            }
            return Err(Error::Transport);
        }
        if let Err(error) = self.outbox.enqueue(event) {
            if let Some(task) = &self.task {
                task.abort();
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn ended(&self) -> bool {
        self.task.as_ref().is_none_or(JoinHandle::is_finished)
    }

    pub fn idle(&self) -> bool {
        self.outbox.offsets.is_empty()
    }

    /// Close only after every opened flow has closed/aborted. The final report
    /// is a best-effort session summary, not a transactional per-mail receipt.
    pub async fn finish(mut self) -> Result<Report, Error> {
        if !self.idle() || self.outbox.failed {
            return Err(Error::Gap);
        }
        self.outbox.sender.take();
        let mut task = self.task.take().ok_or(Error::Transport)?;
        match tokio::time::timeout(Duration::from_secs(5), &mut task).await {
            Ok(Ok(report)) => report,
            _ => {
                task.abort();
                Err(Error::Transport)
            }
        }
    }
}

impl Drop for CaptureUpload {
    fn drop(&mut self) {
        self.outbox.poison();
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outbox(capacity: usize) -> (Outbox, mpsc::Receiver<Result<Bytes, Infallible>>) {
        let (sender, receiver) = mpsc::channel(capacity);
        (
            Outbox { sender: Some(sender), offsets: BTreeMap::new(), last_id: 0, failed: false },
            receiver,
        )
    }

    #[test]
    fn overflow_permanently_poisoned_before_more_ciphertext() {
        let (mut output, _receiver) = outbox(1);
        output.enqueue(Event::Open { id: 1, server_port: 3101 }).expect("open");
        assert_eq!(
            output.enqueue(Event::Data { id: 1, offset: 0, bytes: vec![1] }),
            Err(Error::Capacity)
        );
        assert_eq!(output.enqueue(Event::Open { id: 2, server_port: 5222 }), Err(Error::Gap));
        assert!(output.sender.is_none());
        assert!(output.offsets.is_empty());
    }

    #[test]
    fn closed_receiver_and_explicit_gap_abort_session() {
        let (mut output, receiver) = outbox(4);
        drop(receiver);
        assert_eq!(output.enqueue(Event::Open { id: 1, server_port: 3101 }), Err(Error::Capacity));
        let (mut output, _receiver) = outbox(4);
        assert_eq!(output.enqueue(Event::Gap), Err(Error::Gap));
        assert!(output.failed);
    }

    #[test]
    fn only_exact_offsets_and_independent_monotonic_ids_are_accepted() {
        let (mut output, mut receiver) = outbox(8);
        output.enqueue(Event::Open { id: 5, server_port: 5222 }).expect("open");
        output
            .enqueue(Event::Data { id: 5, offset: 0, bytes: b"synthetic".to_vec() })
            .expect("data");
        output.enqueue(Event::Close { id: 5 }).expect("close");
        assert_eq!(receiver.try_recv().expect("open frame").expect("infallible").len(), 27);
        assert!(output.offsets.is_empty());
        assert_eq!(output.enqueue(Event::Open { id: 5, server_port: 5222 }), Err(Error::Sequence));
        let (mut output, _receiver) = outbox(8);
        output.enqueue(Event::Open { id: 1, server_port: 3101 }).expect("open");
        assert_eq!(
            output.enqueue(Event::Data { id: 1, offset: 1, bytes: vec![1] }),
            Err(Error::Sequence)
        );
    }
}
