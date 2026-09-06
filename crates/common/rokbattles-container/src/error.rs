/// Errors from envelope processing and schema decoding.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The input cannot contain a complete header.
    #[error("ROKB header is truncated")]
    TruncatedHeader,
    /// The magic does not identify a ROKB file.
    #[error("invalid ROKB magic")]
    InvalidMagic,
    /// This reader cannot interpret the envelope version.
    #[error("unsupported ROKB version {0}")]
    UnsupportedVersion(u8),
    /// The flags contain undefined bits.
    #[error("unsupported ROKB flags {0:#04x}")]
    UnsupportedFlags(u8),
    /// Schema ID zero is reserved.
    #[error("ROKB schema ID zero is reserved")]
    InvalidSchema,
    /// The selected decoder does not support this schema ID.
    #[error("unknown ROKB schema {0}")]
    UnknownSchema(u16),
    /// The caller expected a different payload type.
    #[error("expected ROKB schema {expected}, got {actual}")]
    SchemaMismatch {
        /// The required schema ID.
        expected: u16,
        /// The ID in the file.
        actual: u16,
    },
    /// Declared length differs from the bytes following the header.
    #[error("ROKB payload length mismatch")]
    LengthMismatch,
    /// The configured limit, wire length, or host allocation size was exceeded.
    #[error("ROKB payload exceeds the size limit")]
    PayloadTooLarge,
    /// The checksum did not match the unmasked payload.
    #[error("ROKB payload checksum failed")]
    ChecksumMismatch,
    /// A text payload contains invalid UTF-8.
    #[error("invalid ROKB UTF-8 text: {0}")]
    InvalidUtf8(#[from] std::str::Utf8Error),
    /// A JSON payload cannot be read or written.
    #[cfg(any(feature = "read", feature = "write"))]
    #[error("invalid ROKB JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// An application-specific decoder rejected its payload.
    #[error("invalid ROKB payload: {0}")]
    InvalidPayload(String),
}
