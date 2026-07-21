#![forbid(unsafe_code)]

//! Parses AllianceAOOIndividualResults mail reports.

mod body;
mod content;
mod metadata;
mod overview;
mod pairings;
mod results;
mod rewards;

pub use rokbattles_codegen_mail_types::allianceaooindividualresults::AllianceAooIndividualResults;
pub use rokbattles_mail_sdk::{ExtractError, Section};
use rokbattles_mail_sdk::{ProcessError, Processor};
use serde_json::Value;

/// Runs the AllianceAOOIndividualResults parser.
pub fn process(input: &Value) -> Result<AllianceAooIndividualResults, ProcessError> {
    processor().process(input)?.into_typed()
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
    fn process_extracts_expected_sections() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Alliance/Persistent.Mail.102185429177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process(&value).expect("process sample");

        assert_eq!(processed.overview.player_name.as_deref(), Some("Grigvar"));
    }

    #[test]
    fn process_extracts_sparse_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Alliance/Persistent.Mail.6890312417293500508.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process(&value).expect("process sample");

        assert!(processed.overview.player_name.is_none());
    }

    #[test]
    fn process_extracts_empty_stat_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Alliance/Persistent.Mail.91536773174395176822.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process(&value).expect("process sample");

        assert!(processed.overview.total_results.is_none());
    }

    #[test]
    fn process_extracts_sample_without_healing_score() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Alliance/Persistent.Mail.71266849169063933424.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process(&value).expect("process sample");

        assert_eq!(processed.results.healing_score, Some(0));
    }
}
