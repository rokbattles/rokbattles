#![forbid(unsafe_code)]

//! Decoder for `Persistent.Mail` files.
//!
//! A complete file contains a fixed nine-byte header followed by one tagged
//! value. The header starts with `0xff` and stores a little-endian
//! 64-bit checksum in bytes 1 through 8. The checksum uses a wrapping DJB2
//! recurrence over the complete file while treating its checksum bytes as zero.
//!
//! Supported value tags are:
//!
//! - `0x01`: boolean (`u8`)
//! - `0x03`: big-endian IEEE-754 `f64`
//! - `0x04`: UTF-8 string with a little-endian `u32` byte length
//! - `0x05`: table contents ending with the explicit `0xff` terminator
//!
//! Tables are read completely before classification. String-keyed tables become
//! JSON objects, numeric keys `1..N` become JSON arrays, other numeric keys
//! become decimal JSON object keys, and unkeyed sequences remain arrays. Empty
//! tables are represented as `[]`. Mixed or duplicate keys are rejected to avoid
//! silently losing information in JSON.

mod common;
mod decoder;
mod value;

pub use common::DecodeError;
pub use decoder::{decode, decode_value, validate_file};
