//! Field extraction failures and the section context added by a processor.

/// A JSON shape, missing-field, or field-type error reported by an extractor.
///
/// Required helpers distinguish a missing key from a present null or wrong
/// type. Optional helpers treat null as absent. Field names are static labels
/// supplied by the caller, rather than automatically constructed JSON paths.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExtractError {
    /// A value passed to an object-root helper is not a JSON object.
    #[error("expected a JSON object")]
    NotObject,
    /// A required key is absent from the object.
    #[error("missing required field: {field}")]
    MissingField {
        /// The required key supplied to the helper.
        field: &'static str,
    },
    /// A value has the wrong JSON type or cannot be converted to the required integer range.
    #[error("invalid type for {field}; expected {expected}")]
    InvalidFieldType {
        /// The field label supplied by the caller.
        field: &'static str,
        /// A description of the required type, such as `unsigned integer`.
        expected: &'static str,
    },
}

/// A failure to validate section names, join a worker, or extract a section.
///
/// Returned by [`crate::Processor::process`] without partial output. See that
/// method for error ordering and panics that can propagate outside this type.
#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    /// A section name is repeated during validation or output insertion.
    #[error("duplicate processor section: {section}")]
    DuplicateSection {
        /// The repeated section name.
        section: &'static str,
    },
    /// An extractor returned an [`ExtractError`].
    #[error("extractor for {section} failed: {source}")]
    ExtractorFailed {
        /// The section name reported by the extractor.
        section: &'static str,
        /// The original extraction error, preserved as the error source.
        #[source]
        source: ExtractError,
    },
    /// An explicitly joined extractor thread panicked.
    #[error("extractor for {section} panicked")]
    ExtractorPanicked {
        /// The section name reported by the extractor.
        section: &'static str,
    },
}
