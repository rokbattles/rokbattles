//! Privileged capture host. No upload, storage, updater, decoder or user-supplied native paths.
#![deny(unsafe_op_in_unsafe_fn)]

pub mod ownership;

#[cfg(windows)]
pub mod windows;

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod unix;
