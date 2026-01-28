//! Error types for extraction and processing.

use std::error::Error;
use std::fmt;

/// Errors raised when an extractor cannot read the expected data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractError {
    /// The input JSON is not an object.
    NotObject,
    /// A required field was missing.
    MissingField {
        /// The missing field name.
        field: &'static str,
    },
    /// A field existed but had an unexpected type.
    InvalidFieldType {
        /// The field name.
        field: &'static str,
        /// The expected JSON type.
        expected: &'static str,
    },
}

impl fmt::Display for ExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtractError::NotObject => write!(f, "expected a JSON object"),
            ExtractError::MissingField { field } => write!(f, "missing required field: {field}"),
            ExtractError::InvalidFieldType { field, expected } => {
                write!(f, "invalid type for {field}; expected {expected}")
            }
        }
    }
}

impl Error for ExtractError {}

/// Errors raised when running a processor across multiple extractors.
#[derive(Debug)]
pub enum ProcessError {
    /// Two extractors attempted to write to the same section.
    DuplicateSection {
        /// The duplicated section name.
        section: &'static str,
    },
    /// An extractor failed while processing its section.
    ExtractorFailed {
        /// The section name.
        section: &'static str,
        /// The underlying extractor error.
        source: ExtractError,
    },
    /// An extractor panicked while running in parallel.
    ExtractorPanicked {
        /// The section name.
        section: &'static str,
    },
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessError::DuplicateSection { section } => {
                write!(f, "duplicate processor section: {section}")
            }
            ProcessError::ExtractorFailed { section, source } => {
                write!(f, "extractor for {section} failed: {source}")
            }
            ProcessError::ExtractorPanicked { section } => {
                write!(f, "extractor for {section} panicked")
            }
        }
    }
}

impl Error for ProcessError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ProcessError::ExtractorFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}
