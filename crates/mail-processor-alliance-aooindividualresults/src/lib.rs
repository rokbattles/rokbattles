#![forbid(unsafe_code)]

//! Parses AllianceAOOIndividualResults mail reports.

mod body;
mod content;
mod metadata;
mod overview;
mod pairings;
mod results;
mod rewards;

pub use mail_processor_sdk::{ExtractError, Section};
use mail_processor_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Runs the AllianceAOOIndividualResults parser with extractors in parallel.
pub fn process_parallel(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process_parallel(input)
}

/// Runs the AllianceAOOIndividualResults parser in extractor order.
pub fn process_sequential(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process_sequential(input)
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(rewards::RewardsExtractor::new()),
        Box::new(body::BodyExtractor::new()),
        Box::new(overview::OverviewExtractor::new()),
        Box::new(pairings::PairingsExtractor::new()),
        Box::new(results::ResultsExtractor::new()),
    ])
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::Value;

    use super::*;

    #[test]
    fn process_parallel_extracts_expected_sections() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.102185429177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process_parallel(&value).expect("process sample");
        let sections = processed.sections();
        assert!(sections.contains_key("metadata"));
        assert!(sections.contains_key("rewards"));
        assert!(sections.contains_key("body"));
        assert!(sections.contains_key("overview"));
        assert!(sections.contains_key("pairings"));
        assert!(sections.contains_key("results"));
    }

    #[test]
    fn process_parallel_extracts_sparse_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.6890312417293500508.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process_parallel(&value).expect("process sample");
        let sections = processed.sections();
        assert!(sections.contains_key("metadata"));
        assert!(sections.contains_key("rewards"));
        assert!(sections.contains_key("body"));
        assert!(sections.contains_key("overview"));
        assert!(sections.contains_key("pairings"));
        assert!(sections.contains_key("results"));
    }

    #[test]
    fn process_parallel_extracts_empty_stat_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.91536773174395176822.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process_parallel(&value).expect("process sample");
        let sections = processed.sections();
        assert!(sections.contains_key("metadata"));
        assert!(sections.contains_key("rewards"));
        assert!(sections.contains_key("body"));
        assert!(sections.contains_key("overview"));
        assert!(sections.contains_key("pairings"));
        assert!(sections.contains_key("results"));
    }

    #[test]
    fn process_parallel_extracts_sample_without_healing_score() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.71266849169063933424.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process_parallel(&value).expect("process sample");
        let sections = processed.sections();
        assert!(sections.contains_key("metadata"));
        assert!(sections.contains_key("rewards"));
        assert!(sections.contains_key("body"));
        assert!(sections.contains_key("overview"));
        assert!(sections.contains_key("pairings"));
        assert!(sections.contains_key("results"));
        assert_eq!(sections["results"].fields()["healing_score"], serde_json::json!(0));
    }
}
