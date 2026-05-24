//! TCP stream tracking and accepted-fragment extraction.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    framing::{FrameReadError, FrameReader},
    handshake::{Handshake, parse_handshake},
    packet::TcpPayload,
    types::{Direction, StreamId},
};

/// A TCP payload fragment from a stream we can process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamFragment {
    /// Zero-based fragment number among accepted stream payloads.
    pub index: u64,
    /// Direction of the TCP payload bytes.
    pub direction: Direction,
    /// Raw TCP payload bytes, including any frame length prefixes.
    pub payload: Vec<u8>,
}

/// Events emitted by [`StreamTracker`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackerEvent {
    /// The first server frame was the API `8562` handshake.
    StreamAccepted {
        /// TCP connection tuple.
        stream: StreamId,
        /// Keys parsed from the handshake.
        handshake: Handshake,
    },
    /// A TCP payload fragment from a processable stream.
    StreamFragment {
        /// TCP connection tuple.
        stream: StreamId,
        /// Captured TCP payload bytes.
        fragment: StreamFragment,
    },
    /// A stream was rejected before we could process it.
    StreamIgnored {
        /// TCP connection tuple.
        stream: StreamId,
        /// Why this stream was rejected.
        reason: IgnoreReason,
    },
    /// FIN/RST closed a tracked stream.
    StreamEnded {
        /// TCP connection tuple.
        stream: StreamId,
    },
}

/// Why a stream was rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IgnoreReason {
    /// Capture started after the client had already sent data.
    CaptureStartedMidStream,
    /// First complete server frame was not the expected handshake.
    FirstServerFrameWasNotHandshake,
    /// Length-prefixed framing failed.
    InvalidFrameLength {
        /// Length from the frame prefix.
        length: usize,
        /// Largest accepted frame length.
        max: usize,
    },
}

impl std::fmt::Display for IgnoreReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CaptureStartedMidStream => {
                f.write_str("capture appears to have started after the stream was active")
            }
            Self::FirstServerFrameWasNotHandshake => {
                f.write_str("first server frame was not API 8562")
            }
            Self::InvalidFrameLength { length, max } => {
                write!(f, "invalid frame length {length}; max is {max}")
            }
        }
    }
}

/// Tracks already-parsed TCP payloads by connection.
#[derive(Debug)]
pub struct StreamTracker {
    port: u16,
    streams: HashMap<StreamId, CandidateStream>,
}

impl StreamTracker {
    /// Create a tracker for one remote TCP port.
    pub fn new(port: u16) -> Self {
        Self { port, streams: HashMap::new() }
    }

    /// Push one TCP payload into the tracker.
    ///
    /// Client-to-server bytes reject captures that started too late before the
    /// server handshake. After a stream is accepted, both directions are emitted
    /// as fragments so downstream processing can keep the stream cipher in sync.
    pub fn push_packet(&mut self, packet: TcpPayload) -> Vec<TrackerEvent> {
        let Some((stream, direction)) = self.stream_for_packet(&packet) else {
            return Vec::new();
        };

        if packet.payload.is_empty() && !self.streams.contains_key(&stream) {
            return Vec::new();
        }

        let candidate = self
            .streams
            .entry(stream.clone())
            .or_insert_with(|| CandidateStream::new(stream.clone()));
        let mut events = candidate.push_payload(direction, &packet.payload);

        if packet.fin || packet.rst {
            events.push(TrackerEvent::StreamEnded { stream: stream.clone() });
            self.streams.remove(&stream);
        }

        events
    }

    fn stream_for_packet(&self, packet: &TcpPayload) -> Option<(StreamId, Direction)> {
        if packet.destination_port == self.port {
            Some((
                StreamId {
                    client_addr: packet.source_addr,
                    client_port: packet.source_port,
                    server_addr: packet.destination_addr,
                    server_port: packet.destination_port,
                },
                Direction::ClientToServer,
            ))
        } else if packet.source_port == self.port {
            Some((
                StreamId {
                    client_addr: packet.destination_addr,
                    client_port: packet.destination_port,
                    server_addr: packet.source_addr,
                    server_port: packet.source_port,
                },
                Direction::ServerToClient,
            ))
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct CandidateStream {
    id: StreamId,
    state: CandidateState,
    server_reader: FrameReader,
    fragments: u64,
    pending_server_payload: Vec<u8>,
}

impl CandidateStream {
    fn new(id: StreamId) -> Self {
        Self {
            id,
            state: CandidateState::Waiting,
            server_reader: FrameReader::new(),
            fragments: 0,
            pending_server_payload: Vec::new(),
        }
    }

    fn push_payload(&mut self, direction: Direction, payload: &[u8]) -> Vec<TrackerEvent> {
        if matches!(self.state, CandidateState::Ignored) {
            return Vec::new();
        }

        if matches!(self.state, CandidateState::Waiting)
            && direction == Direction::ClientToServer
            && !payload.is_empty()
        {
            self.state = CandidateState::Ignored;
            return vec![TrackerEvent::StreamIgnored {
                stream: self.id.clone(),
                reason: IgnoreReason::CaptureStartedMidStream,
            }];
        }

        if direction == Direction::ClientToServer {
            return self.accepted_fragment(direction, payload).into_iter().collect();
        }

        if matches!(self.state, CandidateState::Waiting) {
            self.pending_server_payload.extend_from_slice(payload);
        }

        let mut events = Vec::new();
        if matches!(self.state, CandidateState::Waiting) {
            let frames = match self.server_reader.push(payload) {
                Ok(frames) => frames,
                Err(FrameReadError::BodyTooLarge { length, max }) => {
                    self.state = CandidateState::Ignored;
                    self.pending_server_payload.clear();
                    return vec![TrackerEvent::StreamIgnored {
                        stream: self.id.clone(),
                        reason: IgnoreReason::InvalidFrameLength { length, max },
                    }];
                }
            };

            let Some(first_body) = frames.first() else {
                return events;
            };
            let Some(handshake) = parse_handshake(first_body) else {
                self.state = CandidateState::Ignored;
                self.pending_server_payload.clear();
                events.push(TrackerEvent::StreamIgnored {
                    stream: self.id.clone(),
                    reason: IgnoreReason::FirstServerFrameWasNotHandshake,
                });
                return events;
            };

            self.state = CandidateState::Accepted;
            events.push(TrackerEvent::StreamAccepted { stream: self.id.clone(), handshake });
            if !self.pending_server_payload.is_empty() {
                let payload = std::mem::take(&mut self.pending_server_payload);
                events.push(self.fragment_event(Direction::ServerToClient, payload));
            }
        } else if let Some(fragment) = self.accepted_fragment(direction, payload) {
            events.push(fragment);
        }

        events
    }

    fn accepted_fragment(&mut self, direction: Direction, payload: &[u8]) -> Option<TrackerEvent> {
        if payload.is_empty() || !matches!(self.state, CandidateState::Accepted) {
            return None;
        }
        Some(self.fragment_event(direction, payload.to_vec()))
    }

    fn fragment_event(&mut self, direction: Direction, payload: Vec<u8>) -> TrackerEvent {
        let index = self.fragments;
        self.fragments += 1;
        TrackerEvent::StreamFragment {
            stream: self.id.clone(),
            fragment: StreamFragment { index, direction, payload },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandidateState {
    Waiting,
    Accepted,
    Ignored,
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;
    use crate::types::CLIENT_PORT;

    const HANDSHAKE_BODY: &[u8] = &[
        0x08, 0xf2, 0x42, 0x12, 0x0c, 0x08, 0x97, 0xd9, 0xd0, 0xaa, 0x02, 0x10, 0xd8, 0xb3, 0x98,
        0xf1, 0x03,
    ];

    #[test]
    fn stream_tracker_should_accept_stream_when_first_server_frame_is_handshake() {
        let mut tracker = StreamTracker::new(CLIENT_PORT);
        let payload = prefixed(HANDSHAKE_BODY);

        let events = tracker.push_packet(packet(CLIENT_PORT, 56_380, payload));

        assert!(matches!(events.first(), Some(TrackerEvent::StreamAccepted { .. })));
    }

    #[test]
    fn stream_tracker_should_emit_only_fragments_after_acceptance() {
        let mut tracker = StreamTracker::new(CLIENT_PORT);
        let _ = tracker.push_packet(packet(CLIENT_PORT, 56_380, prefixed(HANDSHAKE_BODY)));

        let events = tracker.push_packet(packet(56_380, CLIENT_PORT, prefixed(&[0xaa])));

        assert!(matches!(events.first(), Some(TrackerEvent::StreamFragment { .. })));
    }

    #[test]
    fn stream_tracker_should_emit_accepted_fragments_in_both_directions() {
        let mut tracker = StreamTracker::new(CLIENT_PORT);

        let accepted = tracker.push_packet(packet(CLIENT_PORT, 56_380, prefixed(HANDSHAKE_BODY)));
        let client = tracker.push_packet(packet(56_380, CLIENT_PORT, prefixed(&[0xaa])));

        assert!(matches!(
            accepted.get(1),
            Some(TrackerEvent::StreamFragment {
                fragment: StreamFragment { index: 0, direction: Direction::ServerToClient, .. },
                ..
            })
        ));
        assert!(matches!(
            client.first(),
            Some(TrackerEvent::StreamFragment {
                fragment: StreamFragment { index: 1, direction: Direction::ClientToServer, .. },
                ..
            })
        ));
    }

    #[test]
    fn stream_tracker_should_ignore_midstream_client_payload_before_handshake() {
        let mut tracker = StreamTracker::new(CLIENT_PORT);
        let events = tracker.push_packet(packet(56_380, CLIENT_PORT, vec![0, 1, 0xff]));

        assert_eq!(
            events,
            vec![TrackerEvent::StreamIgnored {
                stream: StreamId {
                    client_addr: IpAddr::from(Ipv4Addr::new(10, 0, 0, 1)),
                    client_port: 56_380,
                    server_addr: IpAddr::from(Ipv4Addr::new(10, 0, 0, 2)),
                    server_port: CLIENT_PORT,
                },
                reason: IgnoreReason::CaptureStartedMidStream,
            }]
        );
    }

    fn prefixed(body: &[u8]) -> Vec<u8> {
        let mut payload = Vec::from(u16::try_from(body.len()).unwrap().to_be_bytes());
        payload.extend_from_slice(body);
        payload
    }

    fn packet(source_port: u16, destination_port: u16, payload: Vec<u8>) -> TcpPayload {
        let (source_addr, destination_addr) = if source_port == CLIENT_PORT {
            (IpAddr::from(Ipv4Addr::new(10, 0, 0, 2)), IpAddr::from(Ipv4Addr::new(10, 0, 0, 1)))
        } else {
            (IpAddr::from(Ipv4Addr::new(10, 0, 0, 1)), IpAddr::from(Ipv4Addr::new(10, 0, 0, 2)))
        };

        TcpPayload {
            source_addr,
            source_port,
            destination_addr,
            destination_port,
            fin: false,
            rst: false,
            payload,
        }
    }
}
