use std::{error::Error, fmt};

use mail_processor_sdk::ResolverError;

/// Errors that can occur while processing battle mail payloads.
#[derive(Debug)]
pub enum ProcessError {
    /// The payload had no battles to process.
    NoBattles,
    /// A JSON parse or decode error occurred.
    Json(serde_json::Error),
    /// A resolver step failed while building the output.
    Resolver(ResolverError),
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoBattles => write!(f, "No battles found in mail"),
            Self::Json(err) => write!(f, "json error: {err}"),
            Self::Resolver(err) => write!(f, "{err}"),
        }
    }
}

impl Error for ProcessError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::NoBattles => None,
            Self::Json(err) => Some(err),
            Self::Resolver(err) => Some(err),
        }
    }
}

impl From<ResolverError> for ProcessError {
    fn from(err: ResolverError) -> Self {
        Self::Resolver(err)
    }
}

impl From<serde_json::Error> for ProcessError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}
