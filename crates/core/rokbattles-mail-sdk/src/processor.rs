//! Extractor traits and processor runtime.

use std::collections::HashSet;

use serde_json::Value;

use crate::{ExtractError, ProcessError, ProcessedMail, Section};

/// Pulls one section out of a decoded mail JSON payload.
pub trait Extractor: Send + Sync {
    /// Name of the section in the processed output.
    fn section(&self) -> &'static str;
    /// Pulls this section out of decoded mail JSON.
    fn extract(&self, input: &Value) -> Result<Section, ExtractError>;
}

/// Runs one or more extractors against decoded mail JSON.
#[derive(Default)]
pub struct Processor {
    extractors: Vec<Box<dyn Extractor>>,
}

impl Processor {
    /// Builds a processor from the given extractors.
    pub fn new(extractors: Vec<Box<dyn Extractor>>) -> Self {
        Self { extractors }
    }

    /// Runs extractors in parallel when they do not depend on each other.
    pub fn process(&self, input: &Value) -> Result<ProcessedMail, ProcessError> {
        self.ensure_unique_sections()?;
        let mut results = Vec::with_capacity(self.extractors.len());

        std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(self.extractors.len());
            for extractor in &self.extractors {
                let extractor = extractor.as_ref();
                let section = extractor.section();
                // Independent sections can run at the same time.
                let handle = scope.spawn(move || extractor.extract(input));
                handles.push((section, handle));
            }

            for (section, handle) in handles {
                let result =
                    handle.join().map_err(|_| ProcessError::ExtractorPanicked { section })?;
                results.push((section, result));
            }

            Ok(())
        })?;

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
