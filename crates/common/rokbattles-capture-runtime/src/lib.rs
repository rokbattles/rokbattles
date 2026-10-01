//! Portable capture lifecycle and bounded server-stream transport.
//!
//! Native adapters remain separate. Client-side inputs are control metadata only;
//! this crate has no API that accepts client payload bytes. Only server bytes from
//! ports 3101 and 5222 may enter a stream. Decoder artifacts belong on ingress.
#![forbid(unsafe_code)]

pub mod lifecycle;
pub mod packet;
pub mod reassembly;
pub mod wire;

/// Service ports accepted by the native adapters and the portable runtime.
pub const SERVER_PORTS: [u16; 2] = [3101, 5222];
