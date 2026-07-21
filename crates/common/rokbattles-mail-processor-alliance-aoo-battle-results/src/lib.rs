#![forbid(unsafe_code)]

//! Parses AllianceAOOBattleResults mail reports.

mod alliances;
mod body;
mod content;
mod metadata;
mod overview;
mod participants;

pub use rokbattles_codegen_mail_types::allianceaoobattleresults::AllianceAooBattleResults;
pub use rokbattles_mail_sdk::{ExtractError, Section};
use rokbattles_mail_sdk::{ProcessError, Processor};
use serde_json::Value;

/// Runs the AllianceAOOBattleResults parser.
pub fn process(input: &Value) -> Result<AllianceAooBattleResults, ProcessError> {
    processor().process(input)?.into_typed()
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
            .join("../../../samples/Alliance/Persistent.Mail.102185423177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process(&value).expect("process sample");

        assert_eq!(processed.alliances.len(), 2);
    }
}
