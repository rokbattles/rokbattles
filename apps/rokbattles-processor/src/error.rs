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
    #[error("invalid mail payload: {0}")]
    InvalidMailPayload(String),
    #[error("mail JSON decode failed: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("zstd decode failed: {0}")]
    Decompress(#[from] std::io::Error),
    #[error("unsupported mail type: {0}")]
    UnsupportedMailType(String),
    #[error("processing failed: {0}")]
    Process(#[from] mail_processor_sdk::ProcessError),
    #[error("bson serialization failed: {0}")]
    BsonEncode(#[from] mongodb::bson::ser::Error),
}
