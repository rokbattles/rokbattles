//! Error types for the processor service.

use crate::config::ConfigError;

/// Errors returned by the processor.
#[derive(Debug, thiserror::Error)]
pub enum ProcessorError {
    #[error("configuration error: {0}")]
    Config(#[from] ConfigError),
    #[error("mongodb error: {0}")]
    Mongo(#[from] mongodb::error::Error),
    #[error("mongo uri must include a default database")]
    MissingDatabase,
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    #[error("processed mail is missing its metadata section")]
    MissingProcessedMetadata,
    #[error("unsupported mail compression algorithm: {0}")]
    UnsupportedCompression(String),
    #[error("invalid uncompressed mail size: {0}")]
    InvalidSize(i64),
    #[error("uncompressed mail size mismatch (expected {expected}, found {actual})")]
    SizeMismatch { expected: usize, actual: usize },
    #[error("mail checksum mismatch (expected {expected}, found {actual})")]
    ChecksumMismatch { expected: String, actual: String },
    #[error("invalid mail payload: {0}")]
    InvalidMailPayload(String),
    #[error("binary mail decode failed: {0}")]
    BinaryDecode(#[from] mail_decoder::DecodeError),
    #[error("zstd decode failed: {0}")]
    Decompress(#[from] std::io::Error),
    #[error("unsupported mail type: {0}")]
    UnsupportedMailType(String),
    #[error("processing failed: {0}")]
    Process(#[from] mail_sdk::ProcessError),
    #[error("bson serialization failed: {0}")]
    BsonEncode(#[from] mongodb::bson::ser::Error),
}
