//! Wire-format constants and decoding errors.

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

/// An error encountered while validating or decoding `Persistent.Mail` data.
///
/// Returned by [`crate::decode`], [`crate::decode_value`], and
/// [`crate::validate_file`]. All offsets are zero-based byte offsets into the
/// buffer passed to the public function. For [`crate::decode`], they include
/// the nine-byte file header.
#[derive(Debug, Error, PartialEq)]
pub enum DecodeError {
    /// The buffer contains fewer than nine header bytes.
    #[error("mail file header requires {required} bytes, found {actual}")]
    HeaderTooShort {
        /// The required header length in bytes (9).
        required: usize,
        /// The length of the supplied buffer in bytes.
        actual: usize,
    },
    /// The first byte is not the file marker `0xff`.
    #[error("invalid mail file marker 0x{found:02x}; expected 0x{expected:02x}")]
    InvalidFileMarker {
        /// The expected marker (`0xff`).
        expected: u8,
        /// The first byte of the supplied buffer.
        found: u8,
    },
    /// The stored checksum differs from the checksum computed over the file.
    #[error("mail file checksum mismatch (stored 0x{stored:016x}, computed 0x{computed:016x})")]
    ChecksumMismatch {
        /// The little-endian checksum stored in bytes 1 through 8.
        stored: u64,
        /// The checksum computed with bytes 1 through 8 treated as zero.
        computed: u64,
    },
    /// The buffer ended before a tag or value could be read.
    #[error("unexpected EOF (needed {needed} bytes, had {remaining})")]
    UnexpectedEof {
        /// The number of bytes requested by the read, including any available bytes.
        needed: usize,
        /// The number of unread bytes in the buffer.
        remaining: usize,
    },
    /// A string contains invalid UTF-8.
    #[error("invalid UTF-8 starting at offset {offset}")]
    InvalidUtf8 {
        /// The offset of the string data, immediately after its length prefix.
        /// This identifies the start of the string, not the first invalid byte.
        offset: usize,
    },
    /// A string declares more bytes than remain in the buffer.
    #[error("string length {length} exceeds remaining {remaining} bytes")]
    LengthOutOfBounds {
        /// The string length in bytes, as declared by its length prefix.
        length: usize,
        /// The number of unread bytes in the buffer.
        remaining: usize,
    },
    /// Bytes remain after a complete value has been decoded.
    #[error("trailing bytes after decode ({remaining} bytes)")]
    TrailingBytes {
        /// The number of bytes following the decoded value.
        remaining: usize,
    },
    /// A table would exceed the nesting limit.
    #[error("container nesting exceeds max depth of {limit}")]
    DepthLimitExceeded {
        /// The maximum number of nested tables (128), including the outermost table.
        limit: usize,
    },
    /// A number is NaN or infinite, which JSON cannot represent.
    #[error("non-finite float cannot be represented: {value}")]
    NonFiniteNumber {
        /// The non-finite value read from the buffer.
        value: f64,
    },
    /// A value begins with an unsupported tag.
    #[error("unsupported value tag 0x{tag:02x} at offset {offset}")]
    UnsupportedTag {
        /// The unrecognized tag byte.
        tag: u8,
        /// The offset of the unsupported tag.
        offset: usize,
    },
    /// The buffer ended between table items without a `0xff` terminator.
    #[error("table starting at offset {offset} is missing its 0xff terminator")]
    MissingTableTerminator {
        /// The offset of the table's opening `0x05` tag.
        offset: usize,
    },
    /// A key/value table contains both string and numeric keys.
    #[error("table at offset {offset} mixes string and numeric keys")]
    MixedTableKeyTypes {
        /// The offset of the table's opening `0x05` tag.
        offset: usize,
    },
    /// A key/value table contains a repeated key.
    #[error("table at offset {offset} contains duplicate key {key:?}")]
    DuplicateTableKey {
        /// The offset of the table's opening `0x05` tag.
        offset: usize,
        /// The repeated key as it would appear in the JSON object. Numeric keys
        /// use the text of the normalized JSON number.
        key: String,
    },
}
