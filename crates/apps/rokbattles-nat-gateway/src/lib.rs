//! Installs kernel NAT and passively observes the resulting TCP connections.
//!
//! Forwarding belongs to conntrack and nftables, so stopping this service does
//! not close endpoint sockets. Observation starts with a witnessed handshake;
//! missing ciphertext retires observation of that flow, never its forwarding.
//!
//! `rules` defines the dedicated node's forwarding policy; `capture` receives
//! NFLOG copies; `packet` validates IPv4/TCP; `reassembly` orders server bytes;
//! `observer` runs the shared decoder and `spool` retains pending uploads.
#![cfg(target_os = "linux")]

pub mod capture;
pub mod config;
pub mod observer;
pub mod packet;
pub mod reassembly;
pub mod rules;
pub mod service;
pub mod spool;
