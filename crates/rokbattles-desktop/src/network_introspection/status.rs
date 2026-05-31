use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use serde::Serialize;
use tcp_stream::{StreamId, TrackerEvent};

const ACTIVE_STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(90);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NetworkClientState {
    Disabled,
    Waiting,
    Connected,
    Disconnected,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkStatus {
    pub(super) state: NetworkClientState,
    pub(super) message: Option<String>,
}

impl NetworkStatus {
    pub(super) fn disabled() -> Self {
        Self {
            state: NetworkClientState::Disabled,
            message: Some("Network introspection is disabled.".to_string()),
        }
    }
}

impl Default for NetworkStatus {
    fn default() -> Self {
        Self::disabled()
    }
}

#[derive(Debug, Default)]
pub(super) struct CaptureStatusTracker {
    active_streams: HashMap<StreamId, Instant>,
}

impl CaptureStatusTracker {
    pub(super) fn waiting_status(&mut self) -> Option<NetworkStatus> {
        self.expire_idle_streams();
        if self.active_streams.is_empty() {
            Some(NetworkStatus {
                state: NetworkClientState::Waiting,
                message: Some("Waiting for client. If it is already open, restart it.".into()),
            })
        } else {
            None
        }
    }

    pub(super) fn tracker_status(&mut self, event: &TrackerEvent) -> Option<NetworkStatus> {
        match event {
            TrackerEvent::StreamAccepted { stream, .. } => {
                self.active_streams.insert(stream.clone(), Instant::now());
                Some(NetworkStatus { state: NetworkClientState::Connected, message: None })
            }
            TrackerEvent::StreamFragment { stream, .. } => {
                self.active_streams.insert(stream.clone(), Instant::now());
                None
            }
            TrackerEvent::StreamEnded { stream } => {
                self.active_streams.remove(stream);
                if self.active_streams.is_empty() {
                    Some(NetworkStatus {
                        state: NetworkClientState::Disconnected,
                        message: Some("Disconnected. Waiting for reconnect.".to_string()),
                    })
                } else {
                    None
                }
            }
            TrackerEvent::StreamIgnored { reason, .. } => {
                if self.active_streams.is_empty() {
                    Some(NetworkStatus {
                        state: NetworkClientState::Waiting,
                        message: Some(format!("{reason}. Restart the client.")),
                    })
                } else {
                    None
                }
            }
        }
    }

    fn expire_idle_streams(&mut self) {
        self.active_streams
            .retain(|_stream, last_seen| last_seen.elapsed() < ACTIVE_STREAM_IDLE_TIMEOUT);
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use tcp_stream::{CLIENT_PORT, Handshake, IgnoreReason};

    use super::*;

    #[test]
    fn capture_status_should_not_wait_while_stream_is_active() {
        let mut tracker = CaptureStatusTracker::default();
        let stream = stream_id();

        let connected = tracker
            .tracker_status(&TrackerEvent::StreamAccepted {
                stream: stream.clone(),
                handshake: Handshake { api_id: 8562, key1: 1, key2: 2 },
            })
            .expect("accepted stream should emit connected status");

        assert_eq!(connected.state, NetworkClientState::Connected);
        assert!(tracker.waiting_status().is_none());
    }

    #[test]
    fn capture_status_should_wait_after_last_stream_ends() {
        let mut tracker = CaptureStatusTracker::default();
        let stream = stream_id();
        let _ = tracker.tracker_status(&TrackerEvent::StreamAccepted {
            stream: stream.clone(),
            handshake: Handshake { api_id: 8562, key1: 1, key2: 2 },
        });

        let disconnected = tracker
            .tracker_status(&TrackerEvent::StreamEnded { stream })
            .expect("ending the last stream should emit disconnected status");
        let waiting = tracker.waiting_status().expect("no active streams should allow waiting");

        assert_eq!(disconnected.state, NetworkClientState::Disconnected);
        assert_eq!(waiting.state, NetworkClientState::Waiting);
    }

    #[test]
    fn capture_status_should_ignore_rejected_candidates_while_connected() {
        let mut tracker = CaptureStatusTracker::default();
        let stream = stream_id();
        let _ = tracker.tracker_status(&TrackerEvent::StreamAccepted {
            stream: stream.clone(),
            handshake: Handshake { api_id: 8562, key1: 1, key2: 2 },
        });

        let ignored = tracker.tracker_status(&TrackerEvent::StreamIgnored {
            stream,
            reason: IgnoreReason::CaptureStartedMidStream,
        });

        assert!(ignored.is_none());
    }

    #[test]
    fn capture_status_should_expire_idle_streams() {
        let mut tracker = CaptureStatusTracker::default();
        let stream = stream_id();
        tracker
            .active_streams
            .insert(stream, Instant::now() - ACTIVE_STREAM_IDLE_TIMEOUT - Duration::from_secs(1));

        let waiting = tracker.waiting_status().expect("idle active stream should be expired");

        assert_eq!(waiting.state, NetworkClientState::Waiting);
    }

    fn stream_id() -> StreamId {
        StreamId {
            client_addr: IpAddr::from(Ipv4Addr::new(10, 0, 0, 1)),
            client_port: 56_380,
            server_addr: IpAddr::from(Ipv4Addr::new(10, 0, 0, 2)),
            server_port: CLIENT_PORT,
        }
    }
}
