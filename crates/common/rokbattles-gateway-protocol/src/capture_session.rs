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
const TOMBSTONE_LIFETIME: Duration = Duration::from_secs(90);
const TOMBSTONE_BYTES: usize = 4 * 1024 * 1024;
const TOMBSTONE_EVENTS: usize = 512;
const HANDSHAKE_ONLY_IDLE: Duration = Duration::from_secs(90);
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

struct InputEvent(Event);

impl Drop for InputEvent {
    fn drop(&mut self) {
        if let Event::Data { bytes, .. } = &mut self.0 {
            bytes.zeroize();
        }
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
    decoder: Option<ServerStreamProcessor<'a>>,
    offset: u64,
    context: MailContext,
    touched: Duration,
    frame_started: Duration,
    retired_at: Option<Duration>,
    ignored_bytes: usize,
    ignored_events: usize,
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
        let mut event = InputEvent(event);
        if self.failed {
            return Err(Error::Sequence);
        }

        self.refill(now);
        let result = self.expire(now).and_then(|()| {
            self.accept_inner(std::mem::replace(&mut event.0, Event::Keepalive), now)
        });

        if result.is_err() {
            self.flows.clear();
            self.buffered = 0;
            self.failed = true;
        }
        result
    }

    /// Drop stale flow state during quiet periods without interpreting partial frames.
    pub fn expire(&mut self, now: Duration) -> Result<(), Error> {
        if self.failed {
            return Err(Error::Sequence);
        }
        if self.flows.values().any(|flow| {
            flow.retired_at.is_some_and(|retired| now.saturating_sub(retired) >= TOMBSTONE_LIFETIME)
        }) {
            self.flows.clear();
            self.buffered = 0;
            self.failed = true;
            return Err(Error::Capacity);
        }
        for flow in self.flows.values_mut() {
            let Some(decoder) = flow.decoder.as_ref() else {
                continue;
            };
            let frames = decoder.completed_frames();
            let initial = frames == 0;
            let deadline = if initial { INITIAL_FRAME_TIMEOUT } else { PARTIAL_FRAME_TIMEOUT };
            let expired = now.saturating_sub(flow.touched) >= FLOW_IDLE
                || (frames == 1 && now.saturating_sub(flow.touched) >= HANDSHAKE_ONLY_IDLE)
                || ((initial || decoder.has_incomplete_frame())
                    && now.saturating_sub(flow.frame_started) >= deadline);
            if expired {
                retire(flow, now);
            }
        }
        self.buffered = self
            .flows
            .values()
            .filter_map(|flow| flow.decoder.as_ref())
            .map(ServerStreamProcessor::buffered_bytes)
            .sum();
        Ok(())
    }

    /// Number of live cipher contexts, for bounded status counters only.
    pub fn flow_count(&self) -> usize {
        self.flows.values().filter(|flow| flow.decoder.is_some()).count()
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
                        decoder: Some(ServerStreamProcessor::new(self.artifact).with_frame_limits(
                            FIRST_FRAME_LIMIT,
                            crate::stream::MAX_FRAME_BODY_BYTES,
                        )),
                        offset: 0,
                        context: MailContext::default(),
                        touched: now,
                        frame_started: now,
                        retired_at: None,
                        ignored_bytes: 0,
                        ignored_events: 0,
                    },
                );
                Ok(Vec::new())
            }
            Event::Close { id } | Event::Abort { id, .. } => {
                let flow = self.flows.remove(&id).ok_or(Error::Sequence)?;
                self.buffered = self.buffered.saturating_sub(
                    flow.decoder.as_ref().map_or(0, ServerStreamProcessor::buffered_bytes),
                );
                Ok(Vec::new())
            }
            Event::Gap => Err(Error::Sequence),
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
                if flow.decoder.is_none() {
                    flow.ignored_bytes =
                        flow.ignored_bytes.checked_add(bytes.len()).ok_or(Error::Capacity)?;
                    flow.ignored_events =
                        flow.ignored_events.checked_add(1).ok_or(Error::Capacity)?;
                    if flow.ignored_bytes > TOMBSTONE_BYTES
                        || flow.ignored_events > TOMBSTONE_EVENTS
                    {
                        return Err(Error::Capacity);
                    }
                    self.credit = self.credit.checked_sub(bytes.len()).ok_or(Error::Capacity)?;
                    flow.offset = next;
                    return Ok(Vec::new());
                }
                let decoder = flow.decoder.as_mut().ok_or(Error::Sequence)?;
                let before = decoder.buffered_bytes();
                let frames_before = decoder.completed_frames();
                let was_incomplete = decoder.has_incomplete_frame();
                let maximum_buffered =
                    MAX_BUFFERED.saturating_sub(self.buffered.saturating_sub(before));
                let decoded = match decoder.push_bounded(&bytes, &mut self.credit, maximum_buffered)
                {
                    Ok(decoded) => decoded,
                    Err(
                        crate::stream::StreamError::WorkBudgetExceeded
                        | crate::stream::StreamError::MemoryBudgetExceeded,
                    ) => return Err(Error::Capacity),
                    Err(_) if frames_before == 0 && decoder.completed_frames() == 0 => {
                        // A rejected provisional connection must not interrupt a
                        // different established game channel in this transport.
                        self.buffered = self.buffered.saturating_sub(before);
                        flow.offset = next;
                        retire(flow, now);
                        flow.ignored_bytes = bytes.len();
                        flow.ignored_events = 1;
                        return Ok(Vec::new());
                    }
                    Err(_) => return Err(Error::Protocol),
                };
                self.buffered =
                    self.buffered.saturating_sub(before).saturating_add(decoder.buffered_bytes());
                if self.buffered > MAX_BUFFERED {
                    return Err(Error::Capacity);
                }
                flow.offset = next;
                flow.touched = now;
                if !was_incomplete
                    || decoder.completed_frames() != frames_before
                    || !decoder.has_incomplete_frame()
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
        let milliseconds = elapsed.as_millis();
        if milliseconds == 0 {
            return;
        }
        let refill = milliseconds.saturating_mul(WORK_PER_SECOND as u128) / 1000;
        self.credit = self
            .credit
            .saturating_add(usize::try_from(refill).unwrap_or(usize::MAX))
            .min(WORK_BURST);
        // Preserve sub-millisecond elapsed time instead of losing every fraction
        // on high-frequency small chunks.
        self.credit_time += Duration::from_millis(u64::try_from(milliseconds).unwrap_or(u64::MAX));
    }
}

// A bounded tombstone keeps exact offsets until Close/Abort without retaining
// keys, context or frame bytes. Retired IDs still count against MAX_FLOWS.
fn retire(flow: &mut Flow<'_>, now: Duration) {
    flow.decoder = None;
    flow.context = MailContext::default();
    flow.retired_at = Some(now);
    flow.ignored_bytes = 0;
    flow.ignored_events = 0;
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
        session.accept(Event::Gap, Duration::ZERO).expect_err("gap poisons transport");
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
            let now = Duration::from_secs(if index <= 1 {
                index as u64
            } else {
                (index as u64 - 1) * 700
            });
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

    #[test]
    fn handshake_only_orphan_expires_without_interrupting_active_mail_stream() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut session = Session::new(&artifact, Duration::ZERO);
        open(&mut session, 1);
        open(&mut session, 2);
        let mut frames = test_server_frames(1).into_iter();
        let handshake = frames.next().expect("handshake");
        let login = frames.next().expect("login");
        let mail = frames.next().expect("mail");
        session
            .accept(Event::Data { id: 1, offset: 0, bytes: handshake.clone() }, Duration::ZERO)
            .expect("orphan handshake");
        session
            .accept(Event::Data { id: 2, offset: 0, bytes: handshake.clone() }, Duration::ZERO)
            .expect("real handshake");
        session
            .accept(
                Event::Data { id: 2, offset: handshake.len() as u64, bytes: login.clone() },
                Duration::from_secs(1),
            )
            .expect("login");
        session.accept(Event::Keepalive, Duration::from_secs(91)).expect("expire orphan");
        assert_eq!(session.flow_count(), 1);
        assert!(session.flows.get(&1).expect("tombstone").decoder.is_none());
        assert_eq!(session.flows.get(&1).expect("tombstone").context.player_id, None);
        let ignored = session
            .accept(
                Event::Data { id: 1, offset: handshake.len() as u64, bytes: vec![1, 2, 3] },
                Duration::from_secs(92),
            )
            .expect("discard retired bytes");
        assert!(ignored.is_empty());
        session.accept(Event::Close { id: 1 }, Duration::from_secs(92)).expect("orphan close");
        let mails = session
            .accept(
                Event::Data { id: 2, offset: (handshake.len() + login.len()) as u64, bytes: mail },
                Duration::from_secs(93),
            )
            .expect("real mail continues");
        assert_eq!(mails.len(), 1);
    }

    #[test]
    fn invalid_first_frame_is_per_flow_and_retired_bytes_still_spend_credit() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut session = Session::new(&artifact, Duration::ZERO);
        open(&mut session, 1);
        open(&mut session, 2);
        session
            .accept(Event::Data { id: 1, offset: 0, bytes: vec![0, 1, 0] }, Duration::ZERO)
            .expect("ignore unsupported first frame");
        assert_eq!(session.flow_count(), 1);
        let before = session.credit;
        session
            .accept(Event::Data { id: 1, offset: 3, bytes: vec![0; 64] }, Duration::ZERO)
            .expect("discard");
        assert_eq!(session.credit, before - 64);
        let valid: Vec<u8> = test_server_frames(1).into_iter().flatten().collect();
        assert_eq!(
            session
                .accept(Event::Data { id: 2, offset: 0, bytes: valid }, Duration::ZERO)
                .expect("independent valid stream")
                .len(),
            1
        );
        assert!(!session.failed);
        assert_eq!(
            session
                .accept(Event::Data { id: 1, offset: 3, bytes: vec![0] }, Duration::ZERO)
                .expect_err("retired offsets cannot replay"),
            Error::Sequence
        );
    }

    #[test]
    fn valid_handshake_then_malformed_frame_in_one_chunk_is_not_an_orphan() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut session = Session::new(&artifact, Duration::ZERO);
        open(&mut session, 1);
        let mut bytes = test_server_frames(0).remove(0);
        // An empty encrypted message cannot be a protocol frame.
        bytes.extend([0, 0]);
        assert_eq!(
            session
                .accept(Event::Data { id: 1, offset: 0, bytes }, Duration::ZERO)
                .expect_err("post-handshake corruption"),
            Error::Protocol
        );
        assert!(session.failed);
        assert_eq!(session.flow_count(), 0);
    }

    #[test]
    fn tombstone_absolute_deadline_and_work_caps_cannot_be_refreshed() {
        let artifact = RuntimeArtifact::test_fixture();
        let rejected = || {
            let mut session = Session::new(&artifact, Duration::ZERO);
            open(&mut session, 1);
            session
                .accept(Event::Data { id: 1, offset: 0, bytes: vec![0, 1, 0] }, Duration::ZERO)
                .expect("reject first frame");
            session
        };
        let mut session = rejected();
        session
            .accept(Event::Data { id: 1, offset: 3, bytes: vec![0] }, Duration::from_secs(89))
            .expect("bounded discard");
        assert_eq!(
            session
                .accept(Event::Keepalive, Duration::from_secs(90))
                .expect_err("absolute deadline"),
            Error::Capacity
        );
        assert_eq!(session.flow_count(), 0);
        assert!(session.failed);

        let mut session = rejected();
        for index in 0..511 {
            session
                .accept(Event::Data { id: 1, offset: 3 + index, bytes: vec![0] }, Duration::ZERO)
                .expect("bounded discarded event");
        }
        assert_eq!(
            session
                .accept(Event::Data { id: 1, offset: 514, bytes: vec![0] }, Duration::ZERO)
                .expect_err("event cap"),
            Error::Capacity
        );

        let mut session = rejected();
        for index in 0..255 {
            session
                .accept(
                    Event::Data {
                        id: 1,
                        offset: 3 + index * MAX_CHUNK as u64,
                        bytes: vec![0; MAX_CHUNK],
                    },
                    Duration::ZERO,
                )
                .expect("bounded discarded bytes");
        }
        assert_eq!(
            session
                .accept(
                    Event::Data {
                        id: 1,
                        offset: 3 + 255 * MAX_CHUNK as u64,
                        bytes: vec![0; MAX_CHUNK]
                    },
                    Duration::ZERO
                )
                .expect_err("byte cap"),
            Error::Capacity
        );
    }

    #[test]
    fn frequent_small_events_do_not_lose_refill_time() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut session = Session::new(&artifact, Duration::ZERO);
        session.credit = 0;
        session.refill(Duration::from_micros(500));
        assert_eq!(session.credit, 0);
        session.refill(Duration::from_micros(1000));
        assert_eq!(session.credit, WORK_PER_SECOND / 1000);
    }
}
