#![forbid(unsafe_code)]

//! Parses ScoutReport mail reports.

mod metadata;

pub use rokbattles_codegen_mail_types::scoutreport::ScoutReport;
pub use rokbattles_mail_sdk::{ExtractError, Section};
use rokbattles_mail_sdk::{ProcessError, Processor};
use serde_json::Value;

/// Runs the ScoutReport parser.
pub fn process(input: &Value) -> Result<ScoutReport, ProcessError> {
    processor().process(input)?.into_typed()
}

fn processor() -> Processor {
    Processor::new(vec![Box::new(metadata::MetadataExtractor::new())])
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn process_extracts_metadata_from_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/ScoutReport/Persistent.Mail.136953280177843782931.json");
        let raw = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&raw).expect("parse sample");

        let processed = process(&value).expect("process sample");
        let processed_json = serde_json::to_value(processed).expect("serialize processed");

        assert_eq!(processed_json["metadata"]["mail_id"], json!("136953280177843782931"));
        assert_eq!(processed_json["metadata"]["mail_receiver"], json!("player_71738515"));
        assert_eq!(processed_json["metadata"]["server_id"], json!(1804));
        assert_eq!(processed_json["metadata"]["mail_time"], json!(1778437829182528u64));
        assert_eq!(processed_json.as_object().map(serde_json::Map::len), Some(1));
    }
}
