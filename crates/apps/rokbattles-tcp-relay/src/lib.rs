//! Bidirectional TCP forwarding with fail-open server-stream observation.
//!
//! Each accepted client receives one independent upstream connection. The
//! relay always prioritizes forwarding. Server-stream processing is bounded
//! and is permanently disabled for a connection when its protocol assumptions
//! stop holding.

#![forbid(unsafe_code)]

mod artifact;
mod config;
mod observer;
mod protobuf;
mod relay;
mod stream;
mod uploader;

pub use artifact::{ArtifactError, RuntimeArtifact};
pub use config::{Config, ConfigError};
pub use relay::serve;
pub use uploader::MailUploader;
