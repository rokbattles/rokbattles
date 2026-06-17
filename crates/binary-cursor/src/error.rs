//! Errors reported by the binary cursor.

use thiserror::Error;

/// Errors returned by `binary-cursor`.
#[derive(Debug, Error)]
pub enum DecodeError {
    /// The buffer ended before all required bytes were available.
    #[error("unexpected EOF (needed {needed} bytes, had {remaining})")]
    UnexpectedEof {
        /// Bytes needed to complete the current read.
        needed: usize,
        /// Bytes remaining in the buffer.
        remaining: usize,
    },
    /// A string contained invalid UTF-8 data.
    #[error("invalid UTF-8 starting at offset {offset}")]
    InvalidUtf8 {
        /// Offset where the invalid UTF-8 sequence started.
        offset: usize,
    },
    /// A string length exceeded the remaining buffer length.
    #[error("string length {length} exceeds remaining {remaining} bytes")]
    LengthOutOfBounds {
        /// Declared string length.
        length: usize,
        /// Bytes remaining in the buffer.
        remaining: usize,
    },
    /// Extra bytes remained after decoding a single value.
    #[error("trailing bytes after decode ({remaining} bytes)")]
    TrailingBytes {
        /// Number of bytes left unread.
        remaining: usize,
    },
    /// Recursion depth exceeded the maximum allowed limit.
    #[error("container nesting exceeds max depth of {limit}")]
    DepthLimitExceeded {
        /// Maximum depth allowed.
        limit: usize,
    },
    /// A floating-point value was NaN or infinite and could not be represented.
    #[error("non-finite float cannot be represented: {value}")]
    NonFiniteNumber {
        /// The offending value.
        value: f64,
    },
}
