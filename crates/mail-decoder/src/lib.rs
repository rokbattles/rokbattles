#![forbid(unsafe_code)]

//! Mail binary decoder.
//!
//! # Format
//! Each value starts with a one-byte tag:
//! - `0x01` -> bool (`u8`, non-zero is `true`)
//! - `0x02` -> `f32` (little-endian)
//! - `0x03` -> `f64` (big-endian)
//! - `0x04` -> UTF-8 string (`u32` length, little-endian, then bytes)
//! - `0x05` -> container (object or array)
//! - Anything else -> `null`
//!
//! Objects are sequences of `key -> value` pairs where keys are encoded as
//! strings (tag `0x04`). Objects end when the next tag is not a string tag.
//! In the sample files, an unknown tag (`0xff`) is used as an explicit
//! terminator; the decoder consumes that byte as the end marker.
//!
//! Arrays use the same `0x05` tag. If the first element is not a string tag,
//! the decoder treats the container as an array of values until the terminator.
//!
//! The files in `samples/` also contain a small leading header. If the first
//! parsed value is `null` and trailing bytes remain, the decoder treats the
//! leading bytes as a preamble and scans for the first offset that yields a
//! complete decode without trailing bytes.

mod common;
mod decoder;
mod lossless;

pub use common::DecodeError;
pub use decoder::decode;
pub use lossless::{
    LosslessArray, LosslessContainer, LosslessDocument, LosslessEncodeError, LosslessEntry,
    LosslessObject, LosslessValue, decode_lossless, encode_lossless, lossless_to_json,
};
