use std::path::PathBuf;

use mail_decoder::{DecodeError, LosslessEncodeError};
use mail_processor_sdk::ProcessError;
use thiserror::Error;

/// Errors returned by the `mail-cli` library.
#[derive(Debug, Error)]
pub enum MailCliError {
    /// The input path was expected to be a directory.
    #[error("input path is not a directory: {}", path.display())]
    InvalidInputDir {
        /// Path that failed validation.
        path: PathBuf,
    },
    /// A filesystem read or write failed.
    #[error("I/O error for {}: {source}", path.display())]
    Io {
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
        /// Path involved in the failure.
        path: PathBuf,
    },
    /// Decoding a mail buffer failed.
    #[error("decode failed for {}: {source}", path.display())]
    Decode {
        /// Underlying decoder error.
        #[source]
        source: DecodeError,
        /// Path to the buffer that was being decoded.
        path: PathBuf,
    },
    /// Serializing decoded JSON failed.
    #[error("JSON serialization failed for {}: {source}", path.display())]
    Json {
        /// Underlying serializer error.
        #[source]
        source: serde_json::Error,
        /// Output path tied to the failure.
        path: PathBuf,
    },
    /// Processing decoded mail JSON failed.
    #[error("processing failed for {}: {source}", path.display())]
    Process {
        /// Underlying processor error.
        #[source]
        source: ProcessError,
        /// Path tied to the processing failure.
        path: PathBuf,
    },
    /// Parsing lossless JSON input failed.
    #[error("lossless JSON parse failed for {}: {source}", path.display())]
    LosslessJson {
        /// Underlying JSON parse error.
        #[source]
        source: serde_json::Error,
        /// Path to the lossless JSON input.
        path: PathBuf,
    },
    /// Lossless JSON did not match the expected shape.
    #[error("lossless JSON format error for {}: {message}", path.display())]
    LosslessFormat {
        /// Description of the format problem.
        message: String,
        /// Path to the lossless JSON input.
        path: PathBuf,
    },
    /// Encoding a lossless document back into bytes failed.
    #[error("lossless JSON encode failed for {}: {source}", path.display())]
    LosslessEncode {
        /// Underlying encoder error.
        #[source]
        source: LosslessEncodeError,
        /// Path to the lossless JSON input.
        path: PathBuf,
    },
    /// The input path was neither a file nor a directory.
    #[error("input path is not a file or directory: {}", path.display())]
    InvalidInputPath {
        /// Path that failed validation.
        path: PathBuf,
    },
    /// The input path did not include a usable file name.
    #[error("missing file name for path: {}", path.display())]
    MissingFileName {
        /// Path that failed validation.
        path: PathBuf,
    },
}
