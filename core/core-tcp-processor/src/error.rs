//! Shared processor error type.

/// Errors raised while loading config, reading captures, decoding packets, or writing output.
#[derive(Debug, thiserror::Error)]
pub enum ProcessorError {
    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),
    #[error("MongoDB URI must include a default database")]
    MissingDatabase,
    #[error(transparent)]
    Mongo(#[from] mongodb::error::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    BsonEncode(#[from] mongodb::bson::ser::Error),
    #[error("missing field: {0}")]
    MissingField(&'static str),
    #[error("invalid field: {0}")]
    InvalidField(&'static str),
    #[error("decode failed: {0}")]
    Decode(String),
}
