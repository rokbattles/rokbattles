//! Runs independent section extractors and assembles their output.
//!
//! Section names are checked before any extraction starts. Scoped threads share
//! the input by reference, and results are joined in registration order before
//! being inserted into the output map.

use std::collections::HashSet;

use serde_json::Value;

use crate::{ExtractError, ProcessError, ProcessedMail, Section};

/// Extracts one named section from a shared decoded JSON payload.
///
/// Implementations must be independent: a [`Processor`] runs all extractors on
/// separate threads and does not provide access to another extractor's output.
/// Each implementation validates the input fields it needs. See the
/// [crate example](crate#examples) for a metadata extractor.
pub trait Extractor: Send + Sync {
    /// Returns the output section name.
    ///
    /// Keep this name stable across calls and unique within a processor. It is
    /// read during validation and again when extraction is started.
    fn section(&self) -> &'static str;
    /// Borrows the input and returns an owned section.
    ///
    /// # Errors
    ///
    /// Returns an [`ExtractError`] when required data is missing or has an
    /// unexpected shape. The processor adds the section name to this error.
    fn extract(&self, input: &Value) -> Result<Section, ExtractError>;
}

/// Runs registered extractors concurrently against the same JSON value.
///
/// Each call creates one scoped thread per extractor, including for a single
/// extractor. An empty processor returns empty processed mail. The processor
/// neither checks the input's root shape nor detects its mail type.
#[derive(Default)]
pub struct Processor {
    extractors: Vec<Box<dyn Extractor>>,
}

impl Processor {
    /// Builds a processor in the given registration order.
    ///
    /// Duplicate section names are checked by [`Self::process`], not here.
    pub fn new(extractors: Vec<Box<dyn Extractor>>) -> Self {
        Self { extractors }
    }

    /// Runs every registered extractor and collects its section.
    ///
    /// Extractors must be independent; dependencies are not detected or ordered.
    /// Results are joined in registration order, regardless of completion order.
    /// An extraction error does not cancel other threads, and no partial output
    /// is returned on failure.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::DuplicateSection`] for duplicate names before
    /// launching threads, or if insertion later finds a duplicate. A joined
    /// thread's panic becomes [`ProcessError::ExtractorPanicked`]. If all joins
    /// succeed, the first extraction error in registration order becomes
    /// [`ProcessError::ExtractorFailed`].
    ///
    /// # Panics
    ///
    /// Panics if a scoped thread cannot be spawned or an extractor's `section`
    /// method panics. After a failed join, the scope joins remaining threads;
    /// a panic from one of those threads can also propagate from this method.
    pub fn process(&self, input: &Value) -> Result<ProcessedMail, ProcessError> {
        self.ensure_unique_sections()?;
        let mut results = Vec::with_capacity(self.extractors.len());

        std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(self.extractors.len());
            for extractor in &self.extractors {
                let extractor = extractor.as_ref();
                let section = extractor.section();
                // The scope lets each thread borrow both the extractor and the input
                // without cloning either or requiring a static input lifetime.
                let handle = scope.spawn(move || extractor.extract(input));
                handles.push((section, handle));
            }

            // Preserve registration order for errors even when workers finish out of order.
            for (section, handle) in handles {
                let result =
                    handle.join().map_err(|_| ProcessError::ExtractorPanicked { section })?;
                results.push((section, result));
            }

            Ok(())
        })?;

        // Only inspect extraction errors after joining; a failed join takes precedence.
        let mut processed = ProcessedMail::new();
        for (section, result) in results {
            let data =
                result.map_err(|source| ProcessError::ExtractorFailed { section, source })?;
            if processed.insert(section.to_string(), data).is_some() {
                return Err(ProcessError::DuplicateSection { section });
            }
        }

        Ok(processed)
    }

    fn ensure_unique_sections(&self) -> Result<(), ProcessError> {
        let mut seen = HashSet::new();
        for extractor in &self.extractors {
            let section = extractor.section();
            if !seen.insert(section) {
                return Err(ProcessError::DuplicateSection { section });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{ExtractError, Section};

    #[derive(Debug)]
    struct TestExtractor {
        section_name: &'static str,
    }

    impl Extractor for TestExtractor {
        fn section(&self) -> &'static str {
            self.section_name
        }

        fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
            let mut section = Section::new();
            let value =
                input.get("value").cloned().ok_or(ExtractError::MissingField { field: "value" })?;
            section.insert("value", value);
            Ok(section)
        }
    }

    #[test]
    fn process_collects_sections() {
        let processor = Processor::new(vec![Box::new(TestExtractor { section_name: "one" })]);
        let input = json!({"value": 10});
        let processed = processor.process(&input).unwrap();
        let section = processed.sections().get("one").unwrap();
        assert_eq!(section.fields().get("value").unwrap(), &json!(10));
    }

    #[test]
    fn process_rejects_duplicate_sections() {
        let processor = Processor::new(vec![
            Box::new(TestExtractor { section_name: "dup" }),
            Box::new(TestExtractor { section_name: "dup" }),
        ]);
        let input = json!({"value": 30});
        let err = processor.process(&input).unwrap_err();
        assert!(matches!(err, ProcessError::DuplicateSection { .. }));
    }

    #[derive(Debug)]
    struct ErrorExtractor;

    impl Extractor for ErrorExtractor {
        fn section(&self) -> &'static str {
            "error"
        }

        fn extract(&self, _input: &Value) -> Result<Section, ExtractError> {
            Err(ExtractError::MissingField { field: "value" })
        }
    }

    #[derive(Debug)]
    struct PanicExtractor;

    impl Extractor for PanicExtractor {
        fn section(&self) -> &'static str {
            "panic"
        }

        fn extract(&self, _input: &Value) -> Result<Section, ExtractError> {
            panic!("boom");
        }
    }

    #[test]
    fn process_propagates_extractor_errors() {
        let processor = Processor::new(vec![Box::new(ErrorExtractor)]);
        let input = json!({"value": 10});
        let err = processor.process(&input).unwrap_err();
        assert!(matches!(err, ProcessError::ExtractorFailed { section: "error", .. }));
    }

    #[test]
    fn process_reports_panics() {
        let processor = Processor::new(vec![
            Box::new(TestExtractor { section_name: "one" }),
            Box::new(PanicExtractor),
        ]);
        let input = json!({"value": 10});
        let err = processor.process(&input).unwrap_err();
        assert!(matches!(err, ProcessError::ExtractorPanicked { section: "panic" }));
    }
}
