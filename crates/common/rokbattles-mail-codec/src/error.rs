//! Errors returned while reading or writing persistent mail.

use thiserror::Error;

/// An error encountered while validating or decoding `Persistent.Mail` data.
///
/// Returned by [`crate::decode`], [`crate::decode_value`], and
/// [`crate::validate_file`]. All offsets are zero-based byte offsets into the
/// buffer passed to the public function. For [`crate::decode`], they include
/// the nine-byte file header.
#[cfg(feature = "read")]
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

/// An error encountered while encoding a `Persistent.Mail` file.
///
/// Returned by [`crate::encode`]. Encoding stops at the first error and discards
/// the partially written file.
#[cfg(feature = "write")]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EncodeError {
    /// The root value or an array element is null.
    ///
    /// Null object fields are omitted before their values are encoded.
    #[error("null cannot be represented by the persistent mail format")]
    NullValue,
    /// A JSON number cannot be converted to a finite `f64`.
    ///
    /// Loss of integer precision alone does not produce this error.
    #[error("number cannot be represented as a finite f64")]
    UnrepresentableNumber,
    /// A string value or object key exceeds `u32::MAX` UTF-8 bytes.
    #[error("string exceeds the persistent mail limit")]
    StringTooLong,
    /// An array element's one-based index cannot be converted to `u64`.
    ///
    /// This checks the integer index before it is written as `f64`; it does
    /// not check whether that floating-point conversion preserves precision.
    #[error("array exceeds the persistent mail limit")]
    ArrayTooLong,
    /// An object or array would exceed the table nesting limit.
    #[error("table nesting exceeds max depth of {limit}")]
    DepthLimitExceeded {
        /// The maximum number of nested tables (128), including the outermost table.
        limit: usize,
    },
    /// The output buffer has no complete checksum field to fill in.
    ///
    /// This indicates an internal invariant was violated: [`crate::encode`]
    /// reserves the header before appending any values.
    #[error("persistent mail file header was not initialized")]
    HeaderNotInitialized,
}
