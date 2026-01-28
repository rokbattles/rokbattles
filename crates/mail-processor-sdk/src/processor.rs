//! Extractor trait and processor orchestration.

use std::collections::HashSet;

use serde_json::Value;

use crate::{ExtractError, ProcessError, ProcessedMail, Section};

/// Extracts a section of processed data from a decoded mail JSON object.
pub trait Extractor: Send + Sync {
    /// The section name used in the processed output.
    fn section(&self) -> &'static str;
    /// Extract the section from the decoded mail JSON.
    fn extract(&self, input: &Value) -> Result<Section, ExtractError>;
}

/// Runs one or more extractors over decoded mail JSON.
#[derive(Default)]
pub struct Processor {
    extractors: Vec<Box<dyn Extractor>>,
}

impl Processor {
    /// Create a processor with the provided extractors.
    pub fn new(extractors: Vec<Box<dyn Extractor>>) -> Self {
        Self { extractors }
    }

    /// Run extractors sequentially in the order provided.
    pub fn process_sequential(&self, input: &Value) -> Result<ProcessedMail, ProcessError> {
        self.ensure_unique_sections()?;
        let mut processed = ProcessedMail::new();
        for extractor in &self.extractors {
            let section = extractor.section();
            let data = extractor
                .extract(input)
                .map_err(|source| ProcessError::ExtractorFailed { section, source })?;
            if processed.insert(section.to_string(), data).is_some() {
                return Err(ProcessError::DuplicateSection { section });
            }
        }
        Ok(processed)
    }

    /// Run extractors in parallel without assuming dependencies between them.
    pub fn process_parallel(&self, input: &Value) -> Result<ProcessedMail, ProcessError> {
        self.ensure_unique_sections()?;
        let mut results = Vec::with_capacity(self.extractors.len());

        std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(self.extractors.len());
            for extractor in &self.extractors {
                let extractor = extractor.as_ref();
                let section = extractor.section();
                // Spawn each extractor so independent sections can run concurrently.
                let handle = scope.spawn(move || extractor.extract(input));
                handles.push((section, handle));
            }

            for (section, handle) in handles {
                let result = handle
                    .join()
                    .map_err(|_| ProcessError::ExtractorPanicked { section })?;
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
    use super::*;
    use crate::{ExtractError, Section};
    use serde_json::json;

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
            let value = input
                .get("value")
                .cloned()
                .ok_or(ExtractError::MissingField { field: "value" })?;
            section.insert("value", value);
            Ok(section)
        }
    }

    #[test]
    fn process_sequential_collects_sections() {
        let processor = Processor::new(vec![Box::new(TestExtractor {
            section_name: "one",
        })]);
        let input = json!({"value": 10});
        let processed = processor.process_sequential(&input).unwrap();
        let section = processed.sections().get("one").unwrap();
        assert_eq!(section.fields().get("value").unwrap(), &json!(10));
    }

    #[test]
    fn process_parallel_collects_sections() {
        let processor = Processor::new(vec![Box::new(TestExtractor {
            section_name: "one",
        })]);
        let input = json!({"value": 20});
        let processed = processor.process_parallel(&input).unwrap();
        let section = processed.sections().get("one").unwrap();
        assert_eq!(section.fields().get("value").unwrap(), &json!(20));
    }

    #[test]
    fn process_rejects_duplicate_sections() {
        let processor = Processor::new(vec![
            Box::new(TestExtractor {
                section_name: "dup",
            }),
            Box::new(TestExtractor {
                section_name: "dup",
            }),
        ]);
        let input = json!({"value": 30});
        let err = processor.process_sequential(&input).unwrap_err();
        assert!(matches!(err, ProcessError::DuplicateSection { .. }));
    }
}
