//! Decodes ordered server bytes and sends raw mail entries to ingress.
//!
//! The NAT gateway uses this crate for framing, cipher state, artifact
//! validation, and the authenticated multipart upload contract. Callers must supply every
//! server byte in order, starting with the handshake. A TCP capture gap cannot
//! be repaired by starting a decoder in the middle of encrypted data.
//!
//! `artifact` validates protocol descriptors; `stream` frames and decrypts;
//! `protobuf` reads bounded messages; `uploader` sends extracted entries.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
mod artifact;
mod protobuf;
pub mod stream;
pub mod uploader;
pub use artifact::{ArtifactError, RuntimeArtifact};
pub use uploader::MailUploader;
