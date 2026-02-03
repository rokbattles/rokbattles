use std::path::PathBuf;

use mail_decoder::{DecodeError, LosslessEncodeError};
use mail_processor_sdk::ProcessError;

/// Errors that can occur while decoding a directory.
#[derive(Debug)]
pub enum MailCliError {
    /// The input path was not a directory.
    InvalidInputDir {
        /// The offending path.
        path: PathBuf,
    },
    /// Failed to read or write from the filesystem.
    Io {
        /// The underlying I/O error.
        source: std::io::Error,
        /// Path associated with the I/O failure.
        path: PathBuf,
    },
    /// Failed to decode a mail buffer.
    Decode {
        /// The decoder error.
        source: DecodeError,
        /// Path to the buffer being decoded.
        path: PathBuf,
    },
    /// Failed to serialize the decoded JSON value.
    Json {
        /// The serializer error.
        source: serde_json::Error,
        /// Path associated with the output.
        path: PathBuf,
    },
    /// Failed to process decoded mail JSON.
    Process {
        /// The processor error.
        source: ProcessError,
        /// Path associated with the processing failure.
        path: PathBuf,
    },
    /// Failed to parse lossless JSON input.
    LosslessJson {
        /// The serializer error.
        source: serde_json::Error,
        /// Path associated with the lossless input.
        path: PathBuf,
    },
    /// Lossless JSON payload did not match the expected schema.
    LosslessFormat {
        /// The format error.
        message: String,
        /// Path associated with the lossless input.
        path: PathBuf,
    },
    /// Failed to encode a lossless document back into bytes.
    LosslessEncode {
        /// The encoder error.
        source: LosslessEncodeError,
        /// Path associated with the lossless input.
        path: PathBuf,
    },
    /// The input path was not a file or directory.
    InvalidInputPath {
        /// The offending path.
        path: PathBuf,
    },
    /// The input file did not have a usable file name.
    MissingFileName {
        /// The offending path.
        path: PathBuf,
    },
}

impl std::fmt::Display for MailCliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailCliError::InvalidInputDir { path } => {
                write!(f, "input path is not a directory: {}", path.display())
            }
            MailCliError::Io { source, path } => {
                write!(f, "I/O error for {}: {source}", path.display())
            }
            MailCliError::Decode { source, path } => {
                write!(f, "decode failed for {}: {source}", path.display())
            }
            MailCliError::Json { source, path } => {
                write!(
                    f,
                    "JSON serialization failed for {}: {source}",
                    path.display()
                )
            }
            MailCliError::Process { source, path } => {
                write!(f, "processing failed for {}: {source}", path.display())
            }
            MailCliError::LosslessJson { source, path } => {
                write!(
                    f,
                    "lossless JSON parse failed for {}: {source}",
                    path.display()
                )
            }
            MailCliError::LosslessFormat { message, path } => {
                write!(
                    f,
                    "lossless JSON format error for {}: {message}",
                    path.display()
                )
            }
            MailCliError::LosslessEncode { source, path } => {
                write!(
                    f,
                    "lossless JSON encode failed for {}: {source}",
                    path.display()
                )
            }
            MailCliError::InvalidInputPath { path } => {
                write!(
                    f,
                    "input path is not a file or directory: {}",
                    path.display()
                )
            }
            MailCliError::MissingFileName { path } => {
                write!(f, "missing file name for path: {}", path.display())
            }
        }
    }
}

impl std::error::Error for MailCliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MailCliError::Io { source, .. } => Some(source),
            MailCliError::Decode { source, .. } => Some(source),
            MailCliError::Json { source, .. } => Some(source),
            MailCliError::Process { source, .. } => Some(source),
            MailCliError::LosslessJson { source, .. } => Some(source),
            MailCliError::LosslessEncode { source, .. } => Some(source),
            _ => None,
        }
    }
}
