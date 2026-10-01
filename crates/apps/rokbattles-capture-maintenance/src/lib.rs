//! Installer-owned capture maintenance. Tests never execute native maintenance.
#![deny(unsafe_op_in_unsafe_fn)]

pub mod lifecycle;
pub mod rollback;
#[cfg(windows)]
pub mod windows;
