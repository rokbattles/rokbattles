//! Local control metadata gates independent server-stream generations.
//!
//! Native adapters admit only typed zero-payload client controls. Their metadata
//! is mapped to witnessed handshake generations here, in the unprivileged worker.
//! No client payload is accepted, retained, logged or serialized to ingress.

use std::{collections::BTreeMap, fmt, time::Duration};

use crate::{
    packet::{ClientTcpControl, FlowKey, ServerPacket},
    reassembly::Reassembly,
};

const MAX_FLOWS: usize = 128;
const MAX_PER_TUPLE: usize = 2;
const MAX_BUFFERED: usize = 8 * 1024 * 1024;
const REORDER_WINDOW: u32 = crate::reassembly::MAX_PENDING_BYTES as u32;
const CANDIDATE_IDLE: Duration = Duration::from_secs(30);
const ACTIVE_IDLE: Duration = Duration::from_secs(30 * 60);

/// Header-only local observation. There is intentionally no payload member.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClientControl {
    Syn { key: FlowKey, initial_sequence: u32 },
    Established { key: FlowKey, initial_sequence: u32, server_initial_sequence: u32 },
    Reset { key: FlowKey, initial_sequence: u32 },
    Fin { key: FlowKey, initial_sequence: u32 },
}

impl fmt::Debug for ClientControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Syn { .. } => "Syn",
            Self::Established { .. } => "Established",
            Self::Reset { .. } => "Reset",
            Self::Fin { .. } => "Fin",
        };
        f.debug_struct(name).finish_non_exhaustive()
    }
}

/// A stable retirement reason, suitable for counters rather than packet logs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbortReason {
    Reset,
    CaptureGap,
    Reassembly,
    AmbiguousGeneration,
    MemoryLimit,
    Idle,
    InvalidHandshake,
}

/// Transport events contain only opaque connection-local IDs and server bytes.
#[derive(PartialEq, Eq)]
pub enum Event {
    Open { id: u64, server_port: u16 },
    Data { id: u64, offset: u64, bytes: Vec<u8> },
    Close { id: u64 },
    Abort { id: u64, reason: AbortReason },
    Gap,
    Keepalive,
}

impl fmt::Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open { id, server_port } => {
                f.debug_struct("Open").field("id", id).field("server_port", server_port).finish()
            }
            Self::Data { id, offset, bytes } => f
                .debug_struct("Data")
                .field("id", id)
                .field("offset", offset)
                .field("bytes", &bytes.len())
                .finish(),
            Self::Close { id } => f.debug_struct("Close").field("id", id).finish(),
            Self::Abort { id, reason } => {
                f.debug_struct("Abort").field("id", id).field("reason", reason).finish()
            }
            Self::Gap => f.write_str("Gap"),
            Self::Keepalive => f.write_str("Keepalive"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Generation {
    key: FlowKey,
    client_isn: u32,
}

struct Flow {
    id: u64,
    server_isn: Option<u32>,
    // Exact client sequence evidence, from a zero-payload control or the server's
    // acknowledgement. Never infer a reset's generation from a four-tuple alone.
    client_next: u32,
    client_control_sequence: u32,
    stream: Option<Reassembly>,
    established: bool,
    offset: u64,
    fin: Option<u32>,
    touched: Duration,
}

/// No wall-clock or network dependency. Supply monotonic elapsed time for tests
/// and runtime. Active connections have no short cumulative lifetime/byte limit;
/// memory and idle state are bounded independently of connection age.
#[derive(Default)]
pub struct Observer {
    flows: BTreeMap<Generation, Flow>,
    next_id: u64,
    buffered: usize,
    last_sweep: Duration,
}

impl Observer {
    /// Consume header-only local metadata. Opening a stream requires all three
    /// handshake observations; missing or reordered handshake evidence never
    /// promotes a stream. Later client ACKs may refresh sequence evidence but
    /// cannot create a generation. RST/FIN never attribute by tuple alone.
    pub fn client(&mut self, packet: ClientTcpControl, now: Duration) -> Vec<Event> {
        let mut events = self.expire(now);
        if !packet.is_valid() {
            events.push(self.gap());
            return events;
        }
        let flags = packet.flags & 0x3f;
        if flags == 0x02 {
            let generation = Generation { key: packet.key, client_isn: packet.sequence };
            if self.flows.get(&generation).is_some_and(|flow| flow.established) {
                self.abort(generation, AbortReason::AmbiguousGeneration, &mut events);
            } else {
                events.extend(self.control(
                    ClientControl::Syn { key: packet.key, initial_sequence: packet.sequence },
                    now,
                ));
            }
            return events;
        }

        let reset = flags & 0x04 != 0;
        let matches: Vec<Generation> = self
            .flows
            .iter()
            .filter(|(generation, flow)| {
                if generation.key != packet.key {
                    return false;
                }
                let exact_sequence = packet.sequence == flow.client_next
                    || packet.sequence == flow.client_control_sequence;
                if reset {
                    return exact_sequence
                        && (flags & 0x10 == 0
                            || flow.stream.as_ref().is_some_and(|stream| {
                                within_window(packet.acknowledgement, stream.next_sequence())
                            }));
                }
                if !flow.established {
                    return flags == 0x10
                        && packet.sequence == generation.client_isn.wrapping_add(1)
                        && flow
                            .server_isn
                            .is_some_and(|isn| packet.acknowledgement == isn.wrapping_add(1));
                }
                // FIN without ACK needs exact known sequence evidence. For ACK/FINACK,
                // both bounded sequence spaces must identify one witnessed generation.
                (exact_sequence
                    || (flags & 0x10 != 0
                        && packet.sequence.wrapping_sub(flow.client_next) <= REORDER_WINDOW))
                    && (flags & 0x10 == 0
                        || flow.stream.as_ref().is_some_and(|stream| {
                            within_window(packet.acknowledgement, stream.next_sequence())
                        }))
            })
            .map(|(generation, _)| *generation)
            .collect();

        let [generation] = matches.as_slice() else {
            for generation in matches {
                self.abort(generation, AbortReason::AmbiguousGeneration, &mut events);
            }
            return events;
        };
        let generation = *generation;
        let Some(flow) = self.flows.get_mut(&generation) else {
            return events;
        };
        if reset {
            events.extend(self.control(
                ClientControl::Reset {
                    key: generation.key,
                    initial_sequence: generation.client_isn,
                },
                now,
            ));
        } else if !flow.established {
            if let Some(server_initial_sequence) = flow.server_isn {
                events.extend(self.control(
                    ClientControl::Established {
                        key: generation.key,
                        initial_sequence: generation.client_isn,
                        server_initial_sequence,
                    },
                    now,
                ));
            }
        } else {
            let next = packet.sequence.wrapping_add(u32::from(flags & 0x01 != 0));
            if next.wrapping_sub(flow.client_control_sequence) < 1 << 31 {
                flow.client_control_sequence = next;
            }
            flow.touched = now;
            if flags & 0x01 != 0 {
                events.extend(self.control(
                    ClientControl::Fin {
                        key: generation.key,
                        initial_sequence: generation.client_isn,
                    },
                    now,
                ));
            }
        }
        events
    }

    pub(crate) fn control(&mut self, control: ClientControl, now: Duration) -> Vec<Event> {
        let mut events = self.expire(now);
        let (key, client_isn) = match control {
            ClientControl::Syn { key, initial_sequence }
            | ClientControl::Established { key, initial_sequence, .. }
            | ClientControl::Reset { key, initial_sequence }
            | ClientControl::Fin { key, initial_sequence } => (key, initial_sequence),
        };
        let generation = Generation { key, client_isn };

        if let ClientControl::Syn { .. } = control {
            if !crate::packet::valid_flow_key(key)
                || self.flows.contains_key(&generation)
                || self.flows.len() >= MAX_FLOWS
                || self.flows.keys().filter(|entry| entry.key == key).count() >= MAX_PER_TUPLE
            {
                return events;
            }

            let Some(id) = self.next_id.checked_add(1) else {
                events.push(self.gap());
                return events;
            };

            self.next_id = id;
            self.flows.insert(
                generation,
                Flow {
                    id,
                    server_isn: None,
                    client_next: client_isn.wrapping_add(1),
                    client_control_sequence: client_isn.wrapping_add(1),
                    stream: None,
                    established: false,
                    offset: 0,
                    fin: None,
                    touched: now,
                },
            );
            return events;
        }

        let Some(flow) = self.flows.get_mut(&generation) else {
            return events;
        };
        flow.touched = now;

        match control {
            ClientControl::Established { server_initial_sequence, .. } => {
                if flow.server_isn != Some(server_initial_sequence) {
                    self.abort(generation, AbortReason::InvalidHandshake, &mut events);
                } else if !flow.established {
                    flow.established = true;
                    events.push(Event::Open { id: flow.id, server_port: key.server.port() });
                }
            }
            ClientControl::Reset { .. } => self.abort(generation, AbortReason::Reset, &mut events),
            // Client FIN is a half-close. The server may send more data afterwards.
            ClientControl::Fin { .. } | ClientControl::Syn { .. } => {}
        }

        events
    }

    pub fn server(&mut self, packet: &ServerPacket<'_>, now: Duration) -> Vec<Event> {
        let mut events = self.expire(now);

        if packet.payload.len() > 65_535 {
            events.push(self.gap());
            return events;
        }

        if packet.flags & 0x17 == 0x12 {
            let generation =
                Generation { key: packet.key, client_isn: packet.acknowledgement.wrapping_sub(1) };
            let Some(flow) = self.flows.get_mut(&generation) else {
                return events;
            };

            if !packet.payload.is_empty()
                || flow.server_isn.is_some_and(|isn| isn != packet.sequence)
            {
                self.abort(generation, AbortReason::InvalidHandshake, &mut events);
                return events;
            }

            if flow.server_isn.is_none() {
                flow.server_isn = Some(packet.sequence);
                flow.stream = Some(Reassembly::new(packet.sequence.wrapping_add(1)));
            }
            flow.touched = now;
            return events;
        }

        let matches: Vec<Generation> = self
            .flows
            .iter()
            .filter(|(generation, flow)| {
                generation.key == packet.key
                    && flow.established
                    && flow.stream.as_ref().is_some_and(|stream| {
                        within_window(packet.sequence, stream.next_sequence())
                    })
            })
            .map(|(generation, _)| *generation)
            .collect();

        let [generation] = matches.as_slice() else {
            for generation in matches {
                self.abort(generation, AbortReason::AmbiguousGeneration, &mut events);
            }
            return events;
        };
        let generation = *generation;

        if packet.flags & 0x04 != 0 {
            self.abort(generation, AbortReason::Reset, &mut events);
            return events;
        }
        if packet.flags & 0x02 != 0 {
            self.abort(generation, AbortReason::InvalidHandshake, &mut events);
            return events;
        }

        let Some(flow) = self.flows.get_mut(&generation) else {
            return events;
        };
        let Some(stream) = flow.stream.as_mut() else {
            return events;
        };
        flow.touched = now;
        if packet.flags & 0x10 != 0
            && packet.acknowledgement.wrapping_sub(flow.client_next) < 1 << 31
        {
            flow.client_next = packet.acknowledgement;
        }

        if packet.flags & 0x01 != 0 {
            let Ok(length) = u32::try_from(packet.payload.len()) else {
                self.abort(generation, AbortReason::Reassembly, &mut events);
                return events;
            };
            let end = packet.sequence.wrapping_add(length);
            if flow.fin.is_some_and(|previous| previous != end) {
                self.abort(generation, AbortReason::Reassembly, &mut events);
                return events;
            }
            flow.fin = Some(end);
        }

        if let Some(terminal) = flow.fin {
            let terminal_distance = terminal.wrapping_sub(stream.next_sequence());
            let start = i64::from(packet.sequence.wrapping_sub(stream.next_sequence()) as i32);
            let end = start.saturating_add(packet.payload.len() as i64);

            if terminal_distance >= 1 << 31
                || end > i64::from(terminal_distance)
                || stream.buffered_beyond(terminal)
            {
                self.abort(generation, AbortReason::Reassembly, &mut events);
                return events;
            }
        }

        let before = stream.buffered();
        let output = stream.push(packet.sequence, packet.payload);
        self.buffered = self.buffered.saturating_sub(before).saturating_add(stream.buffered());

        let Ok(bytes) = output else {
            self.abort(generation, AbortReason::Reassembly, &mut events);
            return events;
        };
        if self.buffered > MAX_BUFFERED {
            self.abort(generation, AbortReason::MemoryLimit, &mut events);
            return events;
        }
        let Some(end) = flow.offset.checked_add(bytes.len() as u64) else {
            self.abort(generation, AbortReason::MemoryLimit, &mut events);
            return events;
        };

        if !bytes.is_empty() {
            events.push(Event::Data { id: flow.id, offset: flow.offset, bytes });
            flow.offset = end;
        }

        // An out-of-order FIN waits for the missing prefix. No bytes after FIN
        // are interpreted as a continuation. Timeout/loss is an Abort, not Close.
        if flow.fin == Some(stream.next_sequence()) {
            let id = flow.id;
            self.remove(generation);
            events.push(Event::Close { id });
        }

        events
    }

    /// A queue drop, native error, source switch or broken transport invalidates
    /// every decoder. The transport must deliver Gap first or close completely.
    pub fn gap(&mut self) -> Event {
        self.flows.clear();
        self.buffered = 0;
        Event::Gap
    }

    /// Call on a timer even when no packets arrive. Backwards time never expires
    /// state early; the caller must use a monotonic clock.
    pub fn expire(&mut self, now: Duration) -> Vec<Event> {
        if now.saturating_sub(self.last_sweep) < Duration::from_secs(1) {
            return Vec::new();
        }
        self.last_sweep = now;

        let stale: Vec<Generation> = self
            .flows
            .iter()
            .filter(|(_, flow)| {
                let timeout = if flow.established { ACTIVE_IDLE } else { CANDIDATE_IDLE };
                now.saturating_sub(flow.touched) >= timeout
            })
            .map(|(generation, _)| *generation)
            .collect();
        let mut events = Vec::new();

        for generation in stale {
            self.abort(generation, AbortReason::Idle, &mut events);
        }
        events
    }

    fn abort(&mut self, generation: Generation, reason: AbortReason, events: &mut Vec<Event>) {
        if let Some(flow) = self.remove(generation)
            && flow.established
        {
            events.push(Event::Abort { id: flow.id, reason });
        }
    }

    fn remove(&mut self, generation: Generation) -> Option<Flow> {
        let flow = self.flows.remove(&generation)?;
        self.buffered =
            self.buffered.saturating_sub(flow.stream.as_ref().map_or(0, Reassembly::buffered));
        Some(flow)
    }
}

fn within_window(sequence: u32, next: u32) -> bool {
    sequence.wrapping_sub(next) <= REORDER_WINDOW || next.wrapping_sub(sequence) <= REORDER_WINDOW
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(client_port: u16) -> FlowKey {
        FlowKey {
            client: ([192, 0, 2, 1], client_port).into(),
            server: ([198, 51, 100, 1], 3101).into(),
        }
    }

    fn packet(
        key: FlowKey,
        sequence: u32,
        acknowledgement: u32,
        flags: u8,
        payload: &[u8],
    ) -> ServerPacket<'_> {
        ServerPacket { key, sequence, acknowledgement, flags, payload }
    }

    fn establish(
        observer: &mut Observer,
        key: FlowKey,
        client_isn: u32,
        server_isn: u32,
        now: Duration,
    ) -> u64 {
        assert!(
            observer
                .control(ClientControl::Syn { key, initial_sequence: client_isn }, now)
                .is_empty()
        );
        assert!(
            observer
                .server(&packet(key, server_isn, client_isn.wrapping_add(1), 0x12, &[]), now)
                .is_empty()
        );
        let events = observer.control(
            ClientControl::Established {
                key,
                initial_sequence: client_isn,
                server_initial_sequence: server_isn,
            },
            now,
        );
        let [Event::Open { id, .. }] = events.as_slice() else {
            panic!("expected Open");
        };
        *id
    }

    fn observed_control(
        key: FlowKey,
        sequence: u32,
        acknowledgement: u32,
        flags: u8,
    ) -> ClientTcpControl {
        ClientTcpControl { key, sequence, acknowledgement, flags }
    }

    fn observed_handshake(observer: &mut Observer, key: FlowKey, client: u32, server: u32) -> u64 {
        let now = Duration::ZERO;
        assert!(observer.client(observed_control(key, client, 0, 0x02), now).is_empty());
        assert!(
            observer
                .server(&packet(key, server, client.wrapping_add(1), 0x12, &[]), now)
                .is_empty()
        );
        let events = observer.client(
            observed_control(key, client.wrapping_add(1), server.wrapping_add(1), 0x10),
            now,
        );
        match events.as_slice() {
            [Event::Open { id, .. }] => *id,
            _ => panic!("handshake should open exactly one stream"),
        }
    }

    #[test]
    fn observed_probe_reset_discards_late_sxntf_and_preserves_other_connections() {
        let mut observer = Observer::default();
        let now = Duration::ZERO;
        let probe = observed_handshake(&mut observer, key(45000), 100, 200);
        let game = observed_handshake(&mut observer, key(45001), 100, 200);
        assert_eq!(
            observer.client(observed_control(key(45000), 101, 0, 0x04), now),
            vec![Event::Abort { id: probe, reason: AbortReason::Reset }]
        );
        assert!(
            observer.server(&packet(key(45000), 201, 101, 0x18, b"late SxNtf"), now).is_empty()
        );
        assert_eq!(
            observer.server(&packet(key(45001), 201, 101, 0x18, b"real SxNtf"), now),
            vec![Event::Data { id: game, offset: 0, bytes: b"real SxNtf".to_vec() }]
        );
    }

    #[test]
    fn observed_tuple_reuse_and_stale_controls_do_not_guess_current_generation() {
        let mut observer = Observer::default();
        let now = Duration::ZERO;
        let old = observed_handshake(&mut observer, key(45000), 100, 200);
        let new = observed_handshake(&mut observer, key(45000), 10_000_000, 20_000_000);
        assert_eq!(
            observer.client(observed_control(key(45000), 101, 201, 0x14), now),
            vec![Event::Abort { id: old, reason: AbortReason::Reset }]
        );
        assert!(observer.client(observed_control(key(45000), 101, 0, 0x04), now).is_empty());
        assert!(observer.server(&packet(key(45000), 201, 101, 0x18, b"stale"), now).is_empty());
        assert_eq!(
            observer.server(&packet(key(45000), 20_000_001, 10_000_001, 0x18, b"new"), now),
            vec![Event::Data { id: new, offset: 0, bytes: b"new".to_vec() }]
        );
    }

    #[test]
    fn observed_handshake_wrap_and_client_fin_preserve_reordered_server_tail() {
        let mut observer = Observer::default();
        let now = Duration::ZERO;
        let id = observed_handshake(&mut observer, key(45000), u32::MAX, u32::MAX - 1);
        assert!(observer.client(observed_control(key(45000), 0, u32::MAX, 0x11), now).is_empty());
        assert!(observer.server(&packet(key(45000), 2, 1, 0x11, b"def"), now).is_empty());
        assert_eq!(
            observer.server(&packet(key(45000), u32::MAX, 1, 0x18, b"abc"), now),
            vec![Event::Data { id, offset: 0, bytes: b"abcdef".to_vec() }, Event::Close { id }]
        );
    }

    #[test]
    fn no_missing_or_reordered_handshake_metadata_can_open_a_stream() {
        let now = Duration::ZERO;
        for omit in 0..3 {
            let mut observer = Observer::default();
            if omit != 0 {
                observer.client(observed_control(key(45000), 100, 0, 0x02), now);
            }
            if omit != 1 {
                observer.server(&packet(key(45000), 200, 101, 0x12, &[]), now);
            }
            if omit != 2 {
                assert!(
                    observer.client(observed_control(key(45000), 101, 201, 0x10), now).is_empty()
                );
            }
            assert!(
                observer
                    .server(&packet(key(45000), 201, 101, 0x18, b"no handshake"), now)
                    .is_empty()
            );
        }
        let mut observer = Observer::default();
        observer.client(observed_control(key(45000), 100, 0, 0x02), now);
        assert!(observer.client(observed_control(key(45000), 101, 201, 0x10), now).is_empty());
        observer.server(&packet(key(45000), 200, 101, 0x12, &[]), now);
        assert!(observer.server(&packet(key(45000), 201, 101, 0x18, b"reordered"), now).is_empty());
        assert!(observer.client(observed_control(key(45000), 102, 201, 0x10), now).is_empty());
        assert!(observer.client(observed_control(key(45000), 101, 202, 0x10), now).is_empty());
    }

    #[test]
    fn server_ack_or_local_ack_updates_exact_reset_sequence_evidence() {
        for server_evidence in [false, true] {
            let mut observer = Observer::default();
            let now = Duration::ZERO;
            let id = observed_handshake(&mut observer, key(45000), u32::MAX - 10, 200);
            if server_evidence {
                assert!(observer.server(&packet(key(45000), 201, 42, 0x10, &[]), now).is_empty());
            } else {
                assert!(
                    observer.client(observed_control(key(45000), 42, 201, 0x10), now).is_empty()
                );
            }
            assert_eq!(
                observer.client(observed_control(key(45000), 42, 201, 0x14), now),
                vec![Event::Abort { id, reason: AbortReason::Reset }]
            );
        }
    }

    #[test]
    fn observed_fin_after_unseen_client_data_is_half_close_and_consumes_one_sequence() {
        let mut observer = Observer::default();
        let now = Duration::ZERO;
        let id = observed_handshake(&mut observer, key(45000), 100, 200);
        observer.server(&packet(key(45000), 201, 900, 0x10, &[]), now);
        assert!(observer.client(observed_control(key(45000), 900, 201, 0x11), now).is_empty());
        assert_eq!(
            observer.server(&packet(key(45000), 201, 901, 0x18, b"tail"), now),
            vec![Event::Data { id, offset: 0, bytes: b"tail".to_vec() }]
        );
        assert_eq!(
            observer.client(observed_control(key(45000), 901, 205, 0x14), now),
            vec![Event::Abort { id, reason: AbortReason::Reset }]
        );
    }

    #[test]
    fn ambiguous_exact_reset_evidence_aborts_every_match_and_half_range_never_matches() {
        let mut observer = Observer::default();
        let now = Duration::ZERO;
        let first = observed_handshake(&mut observer, key(45000), 100, 200);
        let second = observed_handshake(&mut observer, key(45000), 500, 20_000_000);
        observer.server(&packet(key(45000), 201, 900, 0x10, &[]), now);
        observer.server(&packet(key(45000), 20_000_001, 900, 0x10, &[]), now);
        assert!(
            observer
                .client(observed_control(key(45000), 900u32.wrapping_add(1 << 31), 0, 0x04), now)
                .is_empty()
        );
        assert_eq!(
            observer.client(observed_control(key(45000), 900, 0, 0x04), now),
            vec![
                Event::Abort { id: first, reason: AbortReason::AmbiguousGeneration },
                Event::Abort { id: second, reason: AbortReason::AmbiguousGeneration },
            ]
        );
    }

    #[test]
    fn malformed_control_invalidates_streams_and_gap_requires_fresh_handshake() {
        let mut observer = Observer::default();
        let now = Duration::ZERO;
        observed_handshake(&mut observer, key(45000), 100, 200);
        assert_eq!(
            observer.client(observed_control(key(45000), 101, 201, 0x18), now),
            vec![Event::Gap]
        );
        assert!(observer.server(&packet(key(45000), 201, 101, 0x18, b"old"), now).is_empty());
        assert!(observer.client(observed_control(key(45000), 101, 201, 0x10), now).is_empty());
        assert!(observer.flows.is_empty());
        observed_handshake(&mut observer, key(45000), 300, 400);
        assert_eq!(observer.gap(), Event::Gap);
        assert!(observer.client(observed_control(key(45000), 301, 401, 0x10), now).is_empty());
    }

    #[test]
    fn observed_control_flood_stays_within_existing_bounds_and_expires() {
        let mut observer = Observer::default();
        for port in 45000..45500 {
            for sequence in 0..3 {
                observer.client(observed_control(key(port), sequence, 0, 0x02), Duration::ZERO);
            }
        }
        assert_eq!(observer.flows.len(), MAX_FLOWS);
        assert_eq!(
            observer.flows.keys().filter(|generation| generation.key == key(45000)).count(),
            MAX_PER_TUPLE
        );
        assert!(observer.expire(CANDIDATE_IDLE).is_empty());
        assert!(observer.flows.is_empty());
    }

    #[test]
    fn probe_reset_drops_late_handshake_without_affecting_real_connection() {
        let mut observer = Observer::default();
        let now = Duration::ZERO;
        let probe = establish(&mut observer, key(45000), 100, 200, now);
        let real = establish(&mut observer, key(45001), 300, 400, now);

        assert_eq!(
            observer.control(ClientControl::Reset { key: key(45000), initial_sequence: 100 }, now),
            vec![Event::Abort { id: probe, reason: AbortReason::Reset }]
        );
        assert!(
            observer.server(&packet(key(45000), 201, 101, 0x18, b"late SxNtf"), now).is_empty()
        );
        assert_eq!(
            observer.server(&packet(key(45001), 401, 301, 0x18, b"real"), now),
            vec![Event::Data { id: real, offset: 0, bytes: b"real".to_vec() }]
        );
    }

    #[test]
    fn client_fin_is_half_close_and_reordered_server_fin_waits_for_missing_bytes() {
        let mut observer = Observer::default();
        let now = Duration::ZERO;
        let id = establish(&mut observer, key(45000), 100, 200, now);

        assert!(
            observer
                .control(ClientControl::Fin { key: key(45000), initial_sequence: 100 }, now)
                .is_empty()
        );
        assert!(observer.server(&packet(key(45000), 204, 101, 0x11, b"def"), now).is_empty());
        assert_eq!(
            observer.server(&packet(key(45000), 201, 101, 0x18, b"abc"), now),
            vec![Event::Data { id, offset: 0, bytes: b"abcdef".to_vec() }, Event::Close { id },]
        );
    }

    #[test]
    fn bytes_beyond_a_witnessed_fin_abort_without_delivery() {
        let mut observer = Observer::default();
        let now = Duration::ZERO;
        let id = establish(&mut observer, key(45000), 100, 200, now);

        assert!(observer.server(&packet(key(45000), 204, 101, 0x11, b""), now).is_empty());
        assert_eq!(
            observer.server(&packet(key(45000), 201, 101, 0x18, b"too long"), now),
            vec![Event::Abort { id, reason: AbortReason::Reassembly }]
        );
    }

    #[test]
    fn fin_with_data_closes_after_output_and_late_conflicting_fin_aborts() {
        let mut observer = Observer::default();
        let now = Duration::ZERO;
        let id = establish(&mut observer, key(45000), 100, 200, now);
        assert_eq!(
            observer.server(&packet(key(45000), 201, 101, 0x11, b"abc"), now),
            vec![Event::Data { id, offset: 0, bytes: b"abc".to_vec() }, Event::Close { id },]
        );

        let id = establish(&mut observer, key(45001), 300, 400, now);
        observer.server(&packet(key(45001), 401, 301, 0x18, b"abc"), now);
        assert_eq!(
            observer.server(&packet(key(45001), 402, 301, 0x11, b""), now),
            vec![Event::Abort { id, reason: AbortReason::Reassembly }]
        );
    }

    #[test]
    fn reused_tuple_generations_are_independent_and_stale_reset_cannot_close_new_one() {
        let mut observer = Observer::default();
        let now = Duration::ZERO;
        let old = establish(&mut observer, key(45000), 100, 200, now);
        let new = establish(&mut observer, key(45000), 500, 10_000_000, now);

        assert_eq!(
            observer.control(ClientControl::Reset { key: key(45000), initial_sequence: 100 }, now),
            vec![Event::Abort { id: old, reason: AbortReason::Reset }]
        );
        assert_eq!(
            observer.server(&packet(key(45000), 10_000_001, 501, 0x18, b"new"), now),
            vec![Event::Data { id: new, offset: 0, bytes: b"new".to_vec() }]
        );
    }

    #[test]
    fn capture_gap_and_midstream_start_never_stitch_cipher_bytes() {
        let mut observer = Observer::default();
        let now = Duration::ZERO;
        assert!(observer.server(&packet(key(45000), 201, 101, 0x18, b"unknown"), now).is_empty());
        establish(&mut observer, key(45000), 100, 200, now);
        assert_eq!(observer.gap(), Event::Gap);
        assert!(observer.server(&packet(key(45000), 201, 101, 0x18, b"old"), now).is_empty());
    }

    #[test]
    fn active_connection_continues_beyond_ten_minutes_and_sequence_wrap() {
        let mut observer = Observer::default();
        let id = establish(&mut observer, key(45000), u32::MAX, u32::MAX - 1, Duration::ZERO);

        assert_eq!(
            observer
                .server(&packet(key(45000), u32::MAX, 0, 0x18, b"ab"), Duration::from_secs(700)),
            vec![Event::Data { id, offset: 0, bytes: b"ab".to_vec() }]
        );
        assert_eq!(
            observer.server(&packet(key(45000), 1, 0, 0x18, b"cd"), Duration::from_secs(1400)),
            vec![Event::Data { id, offset: 2, bytes: b"cd".to_vec() }]
        );
        assert_eq!(
            observer.expire(Duration::from_secs(1400) + ACTIVE_IDLE),
            vec![Event::Abort { id, reason: AbortReason::Idle }]
        );
    }

    #[test]
    fn ambiguous_overlapping_generations_abort_instead_of_guessing() {
        let mut observer = Observer::default();
        let now = Duration::ZERO;
        let first = establish(&mut observer, key(45000), 100, 200, now);
        let second = establish(&mut observer, key(45000), 300, 400, now);
        assert_eq!(
            observer.server(&packet(key(45000), 401, 301, 0x18, b"ambiguous"), now),
            vec![
                Event::Abort { id: first, reason: AbortReason::AmbiguousGeneration },
                Event::Abort { id: second, reason: AbortReason::AmbiguousGeneration },
            ]
        );
    }

    #[test]
    fn candidate_limits_and_expiry_restore_capacity() {
        let mut observer = Observer::default();
        let now = Duration::ZERO;
        for index in 0..MAX_FLOWS {
            let port = 45000 + u16::try_from(index).expect("port");
            observer.control(ClientControl::Syn { key: key(port), initial_sequence: 1 }, now);
        }
        assert_eq!(observer.flows.len(), MAX_FLOWS);
        observer.control(ClientControl::Syn { key: key(50000), initial_sequence: 1 }, now);
        assert_eq!(observer.flows.len(), MAX_FLOWS);
        assert!(observer.expire(CANDIDATE_IDLE).is_empty());
        assert!(observer.flows.is_empty());
        assert_eq!(observer.buffered, 0);

        for initial_sequence in 0..3 {
            observer
                .control(ClientControl::Syn { key: key(45000), initial_sequence }, CANDIDATE_IDLE);
        }
        assert_eq!(observer.flows.len(), MAX_PER_TUPLE);
    }

    #[test]
    fn aggregate_reorder_limit_retires_only_growing_flow_and_recovers_accounting() {
        let mut observer = Observer::default();
        let now = Duration::ZERO;
        let payload = vec![42; 65_535];
        let mut aborted = false;

        for index in 0..9u16 {
            let tuple = key(45000 + index);
            let id = establish(&mut observer, tuple, 100, 200, now);
            for segment in 0..16u32 {
                let events = observer
                    .server(&packet(tuple, 202 + segment * 65_535, 101, 0x18, &payload), now);
                if events == vec![Event::Abort { id, reason: AbortReason::MemoryLimit }] {
                    aborted = true;
                    break;
                }
            }
            assert!(observer.buffered <= MAX_BUFFERED);
        }
        assert!(aborted);
        assert_eq!(
            observer.buffered,
            observer
                .flows
                .values()
                .map(|flow| flow.stream.as_ref().map_or(0, Reassembly::buffered))
                .sum()
        );
        observer.gap();
        assert_eq!(observer.buffered, 0);
        assert!(observer.flows.is_empty());
    }

    #[test]
    fn conflicting_pending_bytes_and_sparse_segment_flood_abort_cleanly() {
        let mut observer = Observer::default();
        let now = Duration::ZERO;
        let id = establish(&mut observer, key(45000), 100, 200, now);
        assert!(observer.server(&packet(key(45000), 203, 101, 0x18, b"abc"), now).is_empty());
        assert_eq!(
            observer.server(&packet(key(45000), 202, 101, 0x18, b"zzz"), now),
            vec![Event::Abort { id, reason: AbortReason::Reassembly }]
        );
        assert_eq!(observer.buffered, 0);

        let id = establish(&mut observer, key(45001), 300, 400, now);
        for offset in 1..=256u32 {
            assert!(
                observer
                    .server(&packet(key(45001), 401 + offset * 2, 301, 0x18, b"x"), now)
                    .is_empty()
            );
        }
        assert_eq!(
            observer.server(&packet(key(45001), 401 + 257 * 2, 301, 0x18, b"x"), now),
            vec![Event::Abort { id, reason: AbortReason::Reassembly }]
        );
        assert_eq!(observer.buffered, 0);
    }
}
