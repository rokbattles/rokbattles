//! TCP stream tracking and server-frame extraction.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    framing::{FrameReadError, FrameReader},
    handshake::{Handshake, parse_handshake},
    packet::TcpPayload,
    types::{Direction, StreamId},
};

/// A server-to-client frame from a stream we can process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerFrame {
    /// Zero-based frame number among server-to-client frames.
    pub index: u64,
    /// Raw frame body with the two-byte length prefix removed.
    pub body: Vec<u8>,
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
    /// A server-to-client frame from a processable stream.
    ServerFrame {
        /// TCP connection tuple.
        stream: StreamId,
        /// Captured server frame.
        frame: ServerFrame,
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
    /// Client-to-server bytes are only used to detect captures that started too
    /// late. The output is server-to-client only because that is all ingress
    /// stores.
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
    server_frames: u64,
}

impl CandidateStream {
    fn new(id: StreamId) -> Self {
        Self {
            id,
            state: CandidateState::Waiting,
            server_reader: FrameReader::new(),
            server_frames: 0,
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
            return Vec::new();
        }

        let frames = match self.server_reader.push(payload) {
            Ok(frames) => frames,
            Err(FrameReadError::BodyTooLarge { length, max }) => {
                self.state = CandidateState::Ignored;
                return vec![TrackerEvent::StreamIgnored {
                    stream: self.id.clone(),
                    reason: IgnoreReason::InvalidFrameLength { length, max },
                }];
            }
        };

        let mut events = Vec::new();
        for body in frames {
            if matches!(self.state, CandidateState::Waiting) {
                let Some(handshake) = parse_handshake(&body) else {
                    self.state = CandidateState::Ignored;
                    events.push(TrackerEvent::StreamIgnored {
                        stream: self.id.clone(),
                        reason: IgnoreReason::FirstServerFrameWasNotHandshake,
                    });
                    break;
                };

                self.state = CandidateState::Accepted;
                events.push(TrackerEvent::StreamAccepted { stream: self.id.clone(), handshake });
            }

            if matches!(self.state, CandidateState::Accepted) {
                let index = self.server_frames;
                self.server_frames += 1;
                events.push(TrackerEvent::ServerFrame {
                    stream: self.id.clone(),
                    frame: ServerFrame { index, body },
                });
            }
        }

        events
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
    fn stream_tracker_should_emit_only_server_frames_after_acceptance() {
        let mut tracker = StreamTracker::new(CLIENT_PORT);
        let _ = tracker.push_packet(packet(CLIENT_PORT, 56_380, prefixed(HANDSHAKE_BODY)));

        let events = tracker.push_packet(packet(56_380, CLIENT_PORT, prefixed(&[0xaa])));

        assert!(events.is_empty());
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
