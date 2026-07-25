#![forbid(unsafe_code)]

//! Encoder for `Persistent.Mail` files.
//!
//! This is the inverse of `rokbattles-mail-decoder`: it writes a JSON value
//! using the game's tagged value format, then adds the fixed file header and
//! checksum.

mod common;
mod encoder;

pub use common::EncodeError;
pub use encoder::encode;
