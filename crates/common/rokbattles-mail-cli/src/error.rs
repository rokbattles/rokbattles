//! Path-aware failures from directory traversal, decoding, processing, and output.

use std::path::PathBuf;

use rokbattles_mail_decoder::DecodeError;
use rokbattles_mail_sdk::ProcessError;
use thiserror::Error;

/// Errors returned by [`crate::run`], retaining the relevant input or output path.
///
/// The binary prints the display message followed by its source error chain.
/// A failure does not imply that no output was written earlier in the run.
#[derive(Debug, Error)]
pub enum MailCliError {
    /// The input path was expected to be a directory.
    #[error("input path is not a directory: {}", path.display())]
    InvalidInputDir {
        /// Path that failed validation.
        path: PathBuf,
    },
    /// Directory access, input reading, or output creation or writing failed.
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
    /// Serializing decoded JSON or processor output failed.
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
        /// Input file whose decoded value was passed to the processor.
        path: PathBuf,
    },
    /// The input filename was missing or could not be represented as UTF-8.
    #[error("missing file name for path: {}", path.display())]
    MissingFileName {
        /// Path that failed validation.
        path: PathBuf,
    },
}
