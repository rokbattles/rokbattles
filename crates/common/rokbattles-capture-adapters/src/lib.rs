//! Optional, passive native capture. Nothing is loaded or started implicitly.
//!
//! Only server-to-client, unfragmented TCP packets with source port 3101 are returned.
//! The adapters do not decode, reassemble, persist, or send.

#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(all(
    any(target_os = "linux", target_os = "macos", target_os = "windows"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod library;
#[cfg(all(
    any(target_os = "linux", target_os = "macos", target_os = "windows"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod packet;
pub mod pcap;
pub mod windivert;

use std::{fmt, path::PathBuf};

/// Maximum non-jumbo IPv6 packet, plus bounded link-layer framing.
#[cfg(all(
    any(target_os = "linux", target_os = "macos", target_os = "windows"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
const SNAPLEN: usize = 65_575 + 256;

/// A single receive attempt; callers choose their own scheduling and cancellation.
#[derive(Debug, PartialEq, Eq)]
pub enum Receive {
    /// Owned, validated IP packet (without link-layer padding or framing).
    Packet(Vec<u8>),
    /// No packet is available now (pcap is nonblocking).
    Idle,
    /// A packet was rejected by the adapter's defense-in-depth admission check.
    Discarded,
    /// The native source has ended.
    End,
}

/// Actionable errors without requiring the application to load a native backend.
#[derive(Debug)]
pub enum Error {
    UnsupportedPlatform(&'static str),
    InvalidInput(&'static str),
    Library { path: PathBuf, detail: String },
    Symbol { name: &'static str, detail: String },
    PermissionDenied { backend: &'static str, detail: String },
    DriverUnavailable { code: i32 },
    Native { operation: &'static str, detail: String },
    UnsupportedLinkType(i32),
    InvalidPacket(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform(backend) => {
                write!(f, "{backend} is unsupported on this platform")
            }
            Self::InvalidInput(detail) => write!(f, "invalid capture input: {detail}"),
            Self::Library { path, detail } => write!(f, "cannot load {}: {detail}", path.display()),
            Self::Symbol { name, detail } => write!(f, "missing capture symbol {name}: {detail}"),
            Self::PermissionDenied { backend, detail } => {
                write!(f, "{backend} capture permission denied: {detail}")
            }
            Self::DriverUnavailable { code } => write!(
                f,
                "WinDivert driver unavailable (Windows error {code}); no driver was installed"
            ),
            Self::Native { operation, detail } => write!(f, "{operation} failed: {detail}"),
            Self::UnsupportedLinkType(kind) => write!(f, "unsupported pcap link type {kind}"),
            Self::InvalidPacket(detail) => write!(f, "invalid native capture result: {detail}"),
        }
    }
}

impl std::error::Error for Error {}
