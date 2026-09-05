#![forbid(unsafe_code)]

//! Encodes [`serde_json::Value`] as `Persistent.Mail` files.
//!
//! Use [`encode`] to build a complete file in memory, including its header and
//! checksum. The caller can then write the returned bytes to disk or pass them
//! to another component.
//!
//! # Examples
//!
//! Null object fields are omitted from the encoded table:
//!
//! ```
//! use rokbattles_mail_encoder::encode;
//! use serde_json::json;
//!
//! let bytes = encode(&json!({ "unread": true, "body": null }))?;
//! assert_eq!(bytes, encode(&json!({ "unread": true }))?);
//! # Ok::<(), rokbattles_mail_encoder::EncodeError>(())
//! ```
//!
//! # File format
//!
//! A file contains a nine-byte header followed by one tagged value. The first
//! byte is `0xff`; bytes 1 through 8 hold a little-endian `u64` checksum. The
//! checksum starts at `5_381` and applies `hash = hash * 33 + byte`, modulo
//! `2^64`, to the entire file, treating the eight checksum bytes as zero.
//!
//! Each value begins with one of these tags:
//!
//! | Tag | Following bytes |
//! | --- | --- |
//! | `0x01` | One byte: `0` for `false`, `1` for `true`. |
//! | `0x03` | A big-endian IEEE 754 `f64`, including for JSON integers. |
//! | `0x04` | A little-endian `u32` byte length, then the UTF-8 string bytes. |
//! | `0x05` | Alternating tagged keys and values, followed by `0xff`. |
//!
//! # Tables and conversion
//!
//! Objects and arrays both become tables. Object keys are encoded as strings,
//! in the order supplied by the JSON map. Array elements receive numeric keys
//! `1..=N` in element order, where `N` is the array length. These keys let
//! `rokbattles-mail-decoder` distinguish arrays from objects.
//!
//! Some JSON distinctions are lost during encoding:
//!
//! - Object fields with null values are omitted, including in nested objects.
//!   A null at the root or directly in an array is rejected.
//! - Empty objects and arrays both become an empty table, which the decoder
//!   represents as `[]`. An object containing only null fields is also empty
//!   after encoding.
//! - All numbers are converted to `f64`. Large integers can lose precision;
//!   finite rounded values are accepted. Numeric object keys stay strings,
//!   even when their text could be parsed as a number.
//!
//! Encoding a decoded file therefore does not guarantee the original bytes.
//! Tables may nest up to 128 levels, counting the outermost table as one level.

mod common;
mod encoder;

pub use common::EncodeError;
pub use encoder::encode;
