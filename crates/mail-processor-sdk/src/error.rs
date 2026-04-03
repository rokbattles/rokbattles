//! Errors shared by extraction and processor execution.

/// Errors returned when an extractor cannot read the input it expected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExtractError {
    /// The input JSON was not an object.
    #[error("expected a JSON object")]
    NotObject,
    /// A required field was missing.
    #[error("missing required field: {field}")]
    MissingField {
        /// Name of the missing field.
        field: &'static str,
    },
    /// A field was present, but its type did not match.
    #[error("invalid type for {field}; expected {expected}")]
    InvalidFieldType {
        /// Name of the field.
        field: &'static str,
        /// Expected JSON type.
        expected: &'static str,
    },
}

/// Errors returned while a processor is running extractors.
#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    /// Two extractors tried to write the same section.
    #[error("duplicate processor section: {section}")]
    DuplicateSection {
        /// Name of the duplicated section.
        section: &'static str,
    },
    /// An extractor returned an error for its section.
    #[error("extractor for {section} failed: {source}")]
    ExtractorFailed {
        /// Name of the section.
        section: &'static str,
        /// The extractor error that bubbled up.
        #[source]
        source: ExtractError,
    },
    /// An extractor panicked while running in parallel.
    #[error("extractor for {section} panicked")]
    ExtractorPanicked {
        /// Name of the section.
        section: &'static str,
    },
}
