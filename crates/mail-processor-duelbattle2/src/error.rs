use std::{error::Error, fmt};

use mail_processor_sdk::ResolverError;

/// Errors that can occur while processing DuelBattle2 mail sections.
#[derive(Debug)]
pub enum ProcessError {
    /// The mail payload had no sections to process.
    EmptySections,
    /// A resolver step failed while building the output.
    Resolver(ResolverError),
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySections => write!(f, "mail payload has no sections"),
            Self::Resolver(err) => write!(f, "{err}"),
        }
    }
}

impl Error for ProcessError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EmptySections => None,
            Self::Resolver(err) => Some(err),
        }
    }
}

impl From<ResolverError> for ProcessError {
    fn from(err: ResolverError) -> Self {
        Self::Resolver(err)
    }
}
