#![forbid(unsafe_code)]

//! Parses AllianceAOORegistration mail reports.

mod content;
mod metadata;
mod overview;
mod participants;

pub use mail_sdk::{ExtractError, Section};
use mail_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Runs the AllianceAOORegistration parser.
pub fn process(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process(input)
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(overview::OverviewExtractor::new()),
        Box::new(participants::ParticipantsExtractor::new()),
    ])
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::Value;

    use super::*;

    #[test]
    fn process_extracts_expected_sections() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.108726086177768046031.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process(&value).expect("process sample");
        let sections = processed.sections();
        assert!(sections.contains_key("metadata"));
        assert!(sections.contains_key("overview"));
        assert!(sections.contains_key("participants"));
    }

    #[test]
    fn process_infers_missing_participant_member_types() {
        for file_name in [
            "Persistent.Mail.108518435177768053226.json",
            "Persistent.Mail.10864788017776806764.json",
        ] {
            let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../samples/Alliance")
                .join(file_name);
            let json = fs::read_to_string(sample_path).expect("read sample");
            let value: Value = serde_json::from_str(&json).expect("parse sample");

            let processed = process(&value).expect("process sample");
            assert!(processed.sections().contains_key("participants"));
        }
    }
}
