//! Bidirectional TCP forwarding.
//!
//! Each accepted client receives one independent upstream connection. The
//! relay forwards bytes without inspecting, logging, or persisting payloads.

#![forbid(unsafe_code)]

mod config;
mod relay;

pub use config::{Config, ConfigError};
pub use relay::serve;
