//! Bounded, local capture IPC. This protocol is never accepted over HTTP or TCP.
//!
//! Authentication belongs to the OS transport. Neither a PID, SID, address nor a
//! token supplied in a message is an authorization credential. Disconnects,
//! malformed records and transport timeouts invalidate all consumer flow state.
#![deny(unsafe_op_in_unsafe_fn)]

mod wire;
pub use wire::{
    Backend, ClientRequest, MAX_BODY_BYTES, PacketBytes, Record, SessionReader, UnavailableReason,
    read_record, read_request, write_record, write_request,
};

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod unix;
#[cfg(windows)]
pub mod windows;

/// SCM service identity, not a caller-configurable service or executable name.
pub const SERVICE_NAME: &str = "ROKBattlesCapture";
/// No peer may keep a partial frame or a stalled write alive indefinitely.
pub const IO_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);
