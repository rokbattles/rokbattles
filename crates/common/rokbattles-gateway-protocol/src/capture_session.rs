//! Incremental, memory-only desktop capture decoding. One instance per transport.
//!
//! A connection-local flow ID and decoded login metadata are untrusted hints,
//! never authentication or proof of mail ownership. No transport or storage is
//! performed here. The caller must durably handle returned mails before accepting
//! more input, and discard the whole session when its transport ends.

use std::{collections::BTreeMap, fmt, time::Duration};

use rokbattles_capture_runtime::{SERVER_PORTS, lifecycle::Event, wire::MAX_CHUNK};
use zeroize::{Zeroize, Zeroizing};

use crate::{
    RuntimeArtifact,
    stream::{ServerStreamProcessor, StreamEvent},
    uploader::MailContext,
};

const MAX_FLOWS: usize = 8;
const MAX_BUFFERED: usize = 64 * 1024 * 1024;
const MAX_RETURNED_BYTES: usize = 32 * 1024 * 1024;
const MAX_RETURNED_MAILS: usize = 512;
const FIRST_FRAME_LIMIT: usize = 64 * 1024;
const FLOW_IDLE: Duration = Duration::from_secs(30 * 60);
const INITIAL_FRAME_TIMEOUT: Duration = Duration::from_secs(15);
const PARTIAL_FRAME_TIMEOUT: Duration = Duration::from_secs(120);
const WORK_BURST: usize = 64 * 1024 * 1024;
const WORK_PER_SECOND: usize = 2 * 1024 * 1024;

/// Stable errors suitable for a compact transport response (no input excerpts).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// Invalid ID, direction, port, ordering or generation lifecycle.
    #[error("invalid capture sequence")]
    Sequence,
    /// Protocol framing/decryption was incompatible with the configured artifact.
    #[error("unsupported server stream")]
    Protocol,
    /// Bounded flow, memory or work allowance was exhausted.
    #[error("capture capacity exceeded")]
    Capacity,
}

/// One raw mail entity for immediate reconstruction/storage. Drop wipes its bytes.
pub struct CapturedMail {
    /// Untrusted connection-derived reconstruction hints.
    pub context: MailContext,
    /// Raw MailEntity bytes, never the whole server stream.
    pub entry: Zeroizing<Vec<u8>>,
}

impl fmt::Debug for CapturedMail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CapturedMail").field("bytes", &self.entry.len()).finish_non_exhaustive()
    }
}

struct DecodedEvents(Vec<StreamEvent>);

impl Drop for DecodedEvents {
    fn drop(&mut self) {
        for event in &mut self.0 {
            if let StreamEvent::Mails { entries, .. } = event {
                for entry in entries {
                    entry.zeroize();
                }
            }
        }
    }
}

struct Flow<'a> {
    decoder: ServerStreamProcessor<'a>,
    offset: u64,
    context: MailContext,
    touched: Duration,
    frame_started: Duration,
}

/// Session-private decoders, keys, player/server context and incomplete frames.
/// Work credits refill with elapsed monotonic time; active connections have no
/// cumulative lifetime quota. No data can resume after this object is dropped.
pub struct Session<'a> {
    artifact: &'a RuntimeArtifact,
    flows: BTreeMap<u64, Flow<'a>>,
    last_id: u64,
    buffered: usize,
    credit: usize,
    credit_time: Duration,
    failed: bool,
}

impl<'a> Session<'a> {
    /// Start with no decoders and a bounded work burst.
    pub fn new(artifact: &'a RuntimeArtifact, now: Duration) -> Self {
        Self {
            artifact,
            flows: BTreeMap::new(),
            last_id: 0,
            buffered: 0,
            credit: WORK_BURST,
            credit_time: now,
            failed: false,
        }
    }

    /// Consume one bounded wire event and return mails as soon as they decode.
    ///
    /// # Errors
    /// On any error, all session state is discarded. The caller must terminate
    /// this transport and cannot re-submit ciphertext to a fresh decoder.
    pub fn accept(&mut self, event: Event, now: Duration) -> Result<Vec<CapturedMail>, Error> {
        if self.failed {
            return Err(Error::Sequence);
        }

        self.refill(now);
        self.expire(now);
        let result = self.accept_inner(event, now);

        if result.is_err() {
            self.flows.clear();
            self.buffered = 0;
            self.failed = true;
        }
        result
    }

    /// Drop stale flow state during quiet periods without interpreting partial frames.
    pub fn expire(&mut self, now: Duration) {
        self.flows.retain(|_, flow| {
            let initial = flow.decoder.completed_frames() == 0;
            let deadline = if initial { INITIAL_FRAME_TIMEOUT } else { PARTIAL_FRAME_TIMEOUT };
            now.saturating_sub(flow.touched) < FLOW_IDLE
                && (!(initial || flow.decoder.has_incomplete_frame())
                    || now.saturating_sub(flow.frame_started) < deadline)
        });
        self.buffered = self.flows.values().map(|flow| flow.decoder.buffered_bytes()).sum();
    }

    /// Number of live cipher contexts, for bounded status counters only.
    pub fn flow_count(&self) -> usize {
        self.flows.len()
    }

    fn accept_inner(&mut self, event: Event, now: Duration) -> Result<Vec<CapturedMail>, Error> {
        match event {
            Event::Open { id, server_port } => {
                if id <= self.last_id
                    || self.flows.len() >= MAX_FLOWS
                    || !SERVER_PORTS.contains(&server_port)
                {
                    return Err(Error::Sequence);
                }
                self.last_id = id;
                self.flows.insert(
                    id,
                    Flow {
                        decoder: ServerStreamProcessor::new(self.artifact).with_frame_limits(
                            FIRST_FRAME_LIMIT,
                            crate::stream::MAX_FRAME_BODY_BYTES,
                        ),
                        offset: 0,
                        context: MailContext::default(),
                        touched: now,
                        frame_started: now,
                    },
                );
                Ok(Vec::new())
            }
            Event::Close { id } | Event::Abort { id, .. } => {
                let flow = self.flows.remove(&id).ok_or(Error::Sequence)?;
                self.buffered = self.buffered.saturating_sub(flow.decoder.buffered_bytes());
                Ok(Vec::new())
            }
            Event::Gap => {
                self.flows.clear();
                self.buffered = 0;
                Ok(Vec::new())
            }
            Event::Keepalive => Ok(Vec::new()),
            Event::Data { id, offset, bytes } => {
                let bytes = Zeroizing::new(bytes);
                if bytes.is_empty() || bytes.len() > MAX_CHUNK {
                    return Err(Error::Sequence);
                }
                let flow = self.flows.get_mut(&id).ok_or(Error::Sequence)?;
                if flow.offset != offset {
                    return Err(Error::Sequence);
                }
                let next = offset.checked_add(bytes.len() as u64).ok_or(Error::Sequence)?;
                let before = flow.decoder.buffered_bytes();
                let frames_before = flow.decoder.completed_frames();
                let was_incomplete = flow.decoder.has_incomplete_frame();
                let maximum_buffered =
                    MAX_BUFFERED.saturating_sub(self.buffered.saturating_sub(before));
                let decoded = flow
                    .decoder
                    .push_bounded(&bytes, &mut self.credit, maximum_buffered)
                    .map_err(|error| {
                        if matches!(
                            error,
                            crate::stream::StreamError::WorkBudgetExceeded
                                | crate::stream::StreamError::MemoryBudgetExceeded
                        ) {
                            Error::Capacity
                        } else {
                            Error::Protocol
                        }
                    })?;
                self.buffered = self
                    .buffered
                    .saturating_sub(before)
                    .saturating_add(flow.decoder.buffered_bytes());
                if self.buffered > MAX_BUFFERED {
                    return Err(Error::Capacity);
                }
                flow.offset = next;
                flow.touched = now;
                if !was_incomplete
                    || flow.decoder.completed_frames() != frames_before
                    || !flow.decoder.has_incomplete_frame()
                {
                    flow.frame_started = now;
                }

                let mut decoded = DecodedEvents(decoded);
                let mut mails = Vec::new();
                let mut returned_bytes = 0usize;
                for event in &mut decoded.0 {
                    match event {
                        StreamEvent::Login { player_id, server_id } => {
                            if *player_id <= 0 || *server_id <= 0 {
                                return Err(Error::Protocol);
                            }
                            flow.context = MailContext {
                                player_id: Some(*player_id),
                                server_id: Some(*server_id),
                            };
                        }
                        StreamEvent::Mails { server_id, entries, .. } => {
                            if flow.context.server_id.is_none() {
                                flow.context.server_id = server_id.filter(|id| *id > 0);
                            }
                            for entry in entries {
                                let entry = Zeroizing::new(std::mem::take(entry));
                                returned_bytes = returned_bytes.saturating_add(entry.len());
                                if returned_bytes > MAX_RETURNED_BYTES
                                    || mails.len() >= MAX_RETURNED_MAILS
                                {
                                    return Err(Error::Capacity);
                                }
                                mails.push(CapturedMail { context: flow.context.clone(), entry });
                            }
                        }
                    }
                }
                Ok(mails)
            }
        }
    }

    fn refill(&mut self, now: Duration) {
        let elapsed = now.saturating_sub(self.credit_time);
        let refill = elapsed.as_millis().saturating_mul(WORK_PER_SECOND as u128) / 1000;
        self.credit = self
            .credit
            .saturating_add(usize::try_from(refill).unwrap_or(usize::MAX))
            .min(WORK_BURST);
        self.credit_time = self.credit_time.max(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::test_server_frames;

    fn open(session: &mut Session<'_>, id: u64) {
        assert!(
            session
                .accept(Event::Open { id, server_port: 3101 }, Duration::ZERO)
                .expect("open")
                .is_empty()
        );
    }

    #[test]
    fn emits_mails_before_disconnect_across_arbitrary_chunks() {
        let artifact = RuntimeArtifact::test_fixture();
        let bytes: Vec<u8> = test_server_frames(3).into_iter().flatten().collect();
        for chunk_size in 1..bytes.len() {
            let mut session = Session::new(&artifact, Duration::ZERO);
            open(&mut session, 1);
            let mut offset = 0u64;
            let mut count = 0;
            for chunk in bytes.chunks(chunk_size) {
                let mails = session
                    .accept(
                        Event::Data { id: 1, offset, bytes: chunk.to_vec() },
                        Duration::from_secs(1),
                    )
                    .expect("decode");
                for mail in mails {
                    assert_eq!(mail.context.player_id, Some(42));
                    assert_eq!(mail.context.server_id, Some(1804));
                    count += 1;
                }
                offset += chunk.len() as u64;
            }
            assert_eq!(count, 3);
            assert_eq!(session.flow_count(), 1);
        }
    }

    #[test]
    fn gap_offset_conflict_and_disconnect_do_not_reuse_cipher_state() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut session = Session::new(&artifact, Duration::ZERO);
        open(&mut session, 1);
        assert_eq!(
            session
                .accept(Event::Data { id: 1, offset: 1, bytes: vec![0] }, Duration::ZERO)
                .expect_err("wrong offset"),
            Error::Sequence
        );
        assert_eq!(session.flow_count(), 0);
        assert_eq!(
            session
                .accept(Event::Open { id: 2, server_port: 3101 }, Duration::ZERO)
                .expect_err("poisoned transport"),
            Error::Sequence
        );
        let mut session = Session::new(&artifact, Duration::ZERO);
        open(&mut session, 2);
        session.accept(Event::Gap, Duration::ZERO).expect("gap");
        assert_eq!(session.flow_count(), 0);
        assert_eq!(
            session
                .accept(Event::Open { id: 2, server_port: 3101 }, Duration::ZERO)
                .expect_err("reused id"),
            Error::Sequence
        );
    }

    #[test]
    fn partial_frame_close_discards_without_emitting_mail() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut session = Session::new(&artifact, Duration::ZERO);
        open(&mut session, 1);
        assert!(
            session
                .accept(Event::Data { id: 1, offset: 0, bytes: vec![0, 17, 8] }, Duration::ZERO)
                .expect("partial")
                .is_empty()
        );
        assert!(session.accept(Event::Close { id: 1 }, Duration::ZERO).expect("close").is_empty());
        assert_eq!(session.flow_count(), 0);
        assert_eq!(session.buffered, 0);
    }

    #[test]
    fn long_active_connection_refills_work_without_lifetime_quota() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut session = Session::new(&artifact, Duration::ZERO);
        open(&mut session, 1);
        let frames = test_server_frames(3);
        let mut offset = 0;
        for (index, frame) in frames.into_iter().enumerate() {
            let now = Duration::from_secs(index as u64 * 700);
            session.accept(Event::Keepalive, now).expect("alive");
            let length = frame.len();
            session.accept(Event::Data { id: 1, offset, bytes: frame }, now).expect("continuous");
            offset += length as u64;
        }
        assert_eq!(session.flow_count(), 1);
    }

    #[test]
    fn incomplete_frame_abort_cycles_spend_work_and_poison_on_exhaustion() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut session = Session::new(&artifact, Duration::ZERO);
        session.credit = 64;
        let mut partial = vec![0, 100];
        partial.extend([0; 14]);

        for id in 1..=4 {
            open(&mut session, id);
            session
                .accept(Event::Data { id, offset: 0, bytes: partial.clone() }, Duration::ZERO)
                .expect("bounded partial");
            session
                .accept(
                    Event::Abort {
                        id,
                        reason: rokbattles_capture_runtime::lifecycle::AbortReason::CaptureGap,
                    },
                    Duration::ZERO,
                )
                .expect("abort");
        }
        assert_eq!(session.credit, 0);
        open(&mut session, 5);
        assert_eq!(
            session
                .accept(Event::Data { id: 5, offset: 0, bytes: partial }, Duration::ZERO)
                .expect_err("work exhausted"),
            Error::Capacity
        );
        assert!(session.failed);
        assert_eq!(session.flow_count(), 0);
    }

    #[test]
    fn keepalive_cannot_extend_incomplete_handshake_deadline() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut session = Session::new(&artifact, Duration::ZERO);
        open(&mut session, 1);
        session
            .accept(Event::Data { id: 1, offset: 0, bytes: vec![0] }, Duration::ZERO)
            .expect("partial");
        session.accept(Event::Keepalive, Duration::from_secs(10)).expect("transport alive");
        session.accept(Event::Keepalive, Duration::from_secs(16)).expect("transport alive");
        assert_eq!(session.flow_count(), 0);
        assert_eq!(session.buffered, 0);
    }
}
