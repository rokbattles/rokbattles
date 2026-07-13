//! Shared constants and error types for decoders.

use thiserror::Error;

pub(crate) const TAG_BOOL: u8 = 0x01;
pub(crate) const TAG_F32: u8 = 0x02;
pub(crate) const TAG_F64: u8 = 0x03;
pub(crate) const TAG_STRING: u8 = 0x04;
pub(crate) const TAG_OBJECT: u8 = 0x05;

pub(crate) const MAX_DEPTH: usize = 128;

/// Errors returned by [crate::decode].
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

pub(crate) fn is_known_tag(tag: u8) -> bool {
    matches!(tag, TAG_BOOL | TAG_F32 | TAG_F64 | TAG_STRING | TAG_OBJECT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_tags_match() {
        assert!(is_known_tag(TAG_BOOL));
        assert!(is_known_tag(TAG_F32));
        assert!(is_known_tag(TAG_F64));
        assert!(is_known_tag(TAG_STRING));
        assert!(is_known_tag(TAG_OBJECT));
        assert!(!is_known_tag(0xff));
    }
}
