#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! TCP stream capture pieces shared by the desktop app and ingress.
//!
//! The server stream uses a two-byte big-endian body length. The first usable
//! server frame is an unencrypted API `8562` notification.
//!
//! Live packet capture stays behind `pcap-capture`; ingress only needs the JSON
//! types and should not link against libpcap/Npcap.

pub mod framing;
pub mod handshake;
pub mod ingest;
pub mod packet;
pub mod tracker;
pub mod types;

#[cfg(feature = "pcap-capture")]
pub mod capture;

#[cfg(feature = "pcap-capture")]
pub use capture::{
    CaptureConfig, CaptureError, CaptureEvent, CaptureSource, list_devices, run_capture,
    run_capture_until,
};
pub use handshake::{HANDSHAKE_API_ID, Handshake, parse_handshake};
pub use ingest::{
    FragmentPayloadError, TcpStreamBatch, TcpStreamBatchValidation, TcpStreamFragmentUpload,
    ValidatedTcpStreamBatch, ValidatedTcpStreamFragment,
};
pub use packet::{TcpPayload, parse_tcp_packet};
pub use tracker::{IgnoreReason, StreamFragment, StreamTracker, TrackerEvent};
pub use types::{CLIENT_PORT, Direction, StreamId};
