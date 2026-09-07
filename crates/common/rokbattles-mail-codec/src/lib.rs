#![forbid(unsafe_code)]

//! Reads and writes `Persistent.Mail` files as JSON values.
//!
//! # Features
//!
//! No features are enabled by default. Enable `read` for `decode`, `decode_value`,
//! `validate_file`, and `DecodeError`; enable `write` for `encode` and `EncodeError`.
//! The features are independent and may be enabled together.
//!
//! ```toml
//! rokbattles-mail-codec = { version = "1.6.1", features = ["read", "write"] }
//! ```
//!
//! Use `decode` to validate a file's header and checksum and decode its contents.
//! Use `decode_value` for a value without a file header, or `validate_file` to
//! check a file's header and checksum without decoding the payload.
//! Use `encode` to build a complete file in memory, including its header and
//! checksum. The caller can write the returned bytes to disk or pass them on.
//!
//! # Examples
//!
//! A headerless table containing the string key `"ok"` and a boolean value:
//!
//! ```
//! # #[cfg(feature = "read")]
//! # {
//! use rokbattles_mail_codec::decode_value;
//! use serde_json::json;
//!
//! let bytes = b"\x05\x04\x02\x00\x00\x00ok\x01\x01\xff";
//! assert_eq!(decode_value(bytes)?, json!({ "ok": true }));
//! # }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Null object fields are omitted when writing:
//!
//! ```
//! # #[cfg(feature = "write")]
//! # {
//! use rokbattles_mail_codec::encode;
//! use serde_json::json;
//!
//! let bytes = encode(&json!({ "unread": true, "body": null }))?;
//! assert_eq!(bytes, encode(&json!({ "unread": true }))?);
//! # }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # File format
//!
//! A file contains a nine-byte header followed by exactly one tagged value. The
//! first byte is `0xff`; bytes 1 through 8 hold a little-endian `u64` checksum.
//! The checksum starts at `5_381` and applies `hash = hash * 33 + byte`, modulo
//! `2^64`, to the entire file, treating the eight checksum bytes as zero.
//!
//! Each value begins with one of these tags:
//!
//! | Tag | Following bytes |
//! | --- | --- |
//! | `0x01` | One byte: zero is `false`, any other value is `true`. |
//! | `0x03` | A big-endian IEEE 754 `f64`. NaN and infinities are rejected. |
//! | `0x04` | A little-endian `u32` byte length, then that many UTF-8 bytes. |
//! | `0x05` | Tagged values followed by a `0xff` table terminator. |
//!
//! Whole numbers are converted to JSON integers when the integer converts back
//! to the same `f64`; other finite numbers remain floating point. Both `0.0` and
//! `-0.0` become integer zero. There is no supported tag for JSON `null`.
//!
//! # Tables
//!
//! Tables have no separate tags for objects and arrays. The decoder reads their
//! contents and chooses a JSON representation using these rules:
//!
//! - An empty table or a table with an odd number of items becomes an array.
//! - For an even number of items, consecutive items are treated as key/value
//!   pairs if every key is a string or number. Otherwise, all items remain in
//!   an array in their original order.
//! - String keys produce an object. Numeric keys that are exactly `1..=N`,
//!   where `N` is the number of pairs, produce an array sorted by key. Other
//!   numeric keys produce an object using their JSON number text as keys.
//!
//! Key/value tables with mixed string and numeric keys or duplicate keys are
//! rejected to avoid losing entries during conversion. Tables may nest up to
//! 128 levels, counting the outermost table as one level.
//!
//! # Tables and conversion
//!
//! Objects and arrays both become tables. Object keys are encoded as strings,
//! in the order supplied by the JSON map. Array elements receive numeric keys
//! `1..=N` in element order, where `N` is the array length. These keys let
//! the decoder distinguish arrays from objects.
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

#[cfg(any(feature = "read", feature = "write"))]
mod common;
#[cfg(feature = "read")]
mod decoder;
#[cfg(feature = "write")]
mod encoder;
#[cfg(any(feature = "read", feature = "write"))]
mod error;
#[cfg(feature = "read")]
mod value;

#[cfg(feature = "read")]
pub use decoder::{decode, decode_value, validate_file};
#[cfg(feature = "write")]
pub use encoder::encode;
#[cfg(feature = "read")]
pub use error::DecodeError;
#[cfg(feature = "write")]
pub use error::EncodeError;
