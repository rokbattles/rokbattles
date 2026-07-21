//! Shared constants and error types for decoders.

use thiserror::Error;

pub(crate) const TAG_BOOL: u8 = 0x01;
pub(crate) const TAG_F64: u8 = 0x03;
pub(crate) const TAG_STRING: u8 = 0x04;
pub(crate) const TAG_TABLE: u8 = 0x05;
pub(crate) const TABLE_END: u8 = 0xff;

pub(crate) const FILE_MARKER: u8 = 0xff;
pub(crate) const FILE_HEADER_LEN: usize = 9;
pub(crate) const CHECKSUM_SEED: u64 = 0x1505;

pub(crate) const MAX_DEPTH: usize = 128;

/// Errors returned by [`crate::decode`], [`crate::decode_value`], and
/// [`crate::validate_file`].
#[derive(Debug, Error, PartialEq)]
pub enum DecodeError {
    /// The buffer is shorter than the fixed mail file header.
    #[error("mail file header requires {required} bytes, found {actual}")]
    HeaderTooShort {
        /// Required fixed header size.
        required: usize,
        /// Actual buffer size.
        actual: usize,
    },
    /// The required mail file marker was not present.
    #[error("invalid mail file marker 0x{found:02x}; expected 0x{expected:02x}")]
    InvalidFileMarker {
        /// Required marker.
        expected: u8,
        /// Marker read from the file.
        found: u8,
    },
    /// The stored mail file checksum did not match the file bytes.
    #[error("mail file checksum mismatch (stored 0x{stored:016x}, computed 0x{computed:016x})")]
    ChecksumMismatch {
        /// Checksum stored in bytes 1 through 8.
        stored: u64,
        /// Checksum computed with those bytes treated as zero.
        computed: u64,
    },
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
    /// A value tag is not supported by this decoder.
    #[error("unsupported value tag 0x{tag:02x} at offset {offset}")]
    UnsupportedTag {
        /// Unsupported tag byte.
        tag: u8,
        /// Absolute offset within the decoded value buffer.
        offset: usize,
    },
    /// A table ended before its explicit terminator.
    #[error("table starting at offset {offset} is missing its 0xff terminator")]
    MissingTableTerminator {
        /// Offset of the table tag.
        offset: usize,
    },
    /// A table mixed string and numeric keys, which JSON cannot preserve safely.
    #[error("table at offset {offset} mixes string and numeric keys")]
    MixedTableKeyTypes {
        /// Offset of the table tag.
        offset: usize,
    },
    /// A table repeated a key and would lose data in JSON.
    #[error("table at offset {offset} contains duplicate key {key:?}")]
    DuplicateTableKey {
        /// Offset of the table tag.
        offset: usize,
        /// Duplicate key rendered using its JSON object representation.
        key: String,
    },
}
