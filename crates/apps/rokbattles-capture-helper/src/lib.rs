//! Privileged capture host. No upload, storage, updater, decoder or user-supplied native paths.
#![deny(unsafe_op_in_unsafe_fn)]

pub mod ownership;

#[cfg(windows)]
pub mod windows;

#[cfg(any(windows, test))]
#[path = "windows/interfaces.rs"]
pub mod interfaces;
#[cfg(any(windows, test))]
#[path = "windows/socket_evidence.rs"]
pub mod socket_evidence;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod unix;
