#![forbid(unsafe_code)]

//! Parses AllianceAOOBattleResults mail reports.

mod alliances;
mod body;
mod content;
mod metadata;
mod overview;
mod participants;

pub use mail_processor_sdk::{ExtractError, Section};
use mail_processor_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Runs the AllianceAOOBattleResults parser.
pub fn process(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process(input)
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(alliances::AlliancesExtractor::new()),
        Box::new(body::BodyExtractor::new()),
        Box::new(participants::ParticipantsExtractor::new()),
        Box::new(overview::OverviewExtractor::new()),
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
            .join("../../samples/Alliance/Persistent.Mail.102185423177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process(&value).expect("process sample");
        let sections = processed.sections();
        assert!(sections.contains_key("metadata"));
        assert!(sections.contains_key("alliances"));
        assert!(sections.contains_key("body"));
        assert!(sections.contains_key("participants"));
        assert!(sections.contains_key("overview"));
    }
}
