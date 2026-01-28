//! Shared constants and error types for decoders.

pub(crate) const TAG_BOOL: u8 = 0x01;
pub(crate) const TAG_F32: u8 = 0x02;
pub(crate) const TAG_F64: u8 = 0x03;
pub(crate) const TAG_STRING: u8 = 0x04;
pub(crate) const TAG_OBJECT: u8 = 0x05;

pub(crate) const MAX_DEPTH: usize = 128;

/// Errors returned by [crate::decode] and [crate::decode_lossless].
#[derive(Debug)]
pub enum DecodeError {
    /// The buffer ended before all required bytes were available.
    UnexpectedEof {
        /// Bytes needed to complete the current read.
        needed: usize,
        /// Bytes remaining in the buffer.
        remaining: usize,
    },
    /// A string contained invalid UTF-8 data.
    InvalidUtf8 {
        /// Offset where the invalid UTF-8 sequence started.
        offset: usize,
    },
    /// A string length exceeded the remaining buffer length.
    LengthOutOfBounds {
        /// Declared string length.
        length: usize,
        /// Bytes remaining in the buffer.
        remaining: usize,
    },
    /// Extra bytes remained after decoding a single value.
    TrailingBytes {
        /// Number of bytes left unread.
        remaining: usize,
    },
    /// Recursion depth exceeded the maximum allowed limit.
    DepthLimitExceeded {
        /// Maximum depth allowed.
        limit: usize,
    },
    /// A floating-point value was NaN or infinite and could not be represented.
    NonFiniteNumber {
        /// The offending value.
        value: f64,
    },
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::UnexpectedEof { needed, remaining } => {
                write!(f, "unexpected EOF (needed {needed} bytes, had {remaining})")
            }
            DecodeError::InvalidUtf8 { offset } => {
                write!(f, "invalid UTF-8 starting at offset {offset}")
            }
            DecodeError::LengthOutOfBounds { length, remaining } => {
                write!(
                    f,
                    "string length {length} exceeds remaining {remaining} bytes"
                )
            }
            DecodeError::TrailingBytes { remaining } => {
                write!(f, "trailing bytes after decode ({remaining} bytes)")
            }
            DecodeError::DepthLimitExceeded { limit } => {
                write!(f, "object nesting exceeds max depth of {limit}")
            }
            DecodeError::NonFiniteNumber { value } => {
                write!(f, "non-finite float cannot be represented: {value}")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

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
