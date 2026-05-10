#![forbid(unsafe_code)]

//! Parses ScoutReport mail reports.

mod metadata;

pub use mail_processor_sdk::{ExtractError, Section};
use mail_processor_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Runs the ScoutReport parser.
pub fn process(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process(input)
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
    fn process_extracts_metadata_from_samples() {
        let sample_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/ScoutReport");
        let mut processed_count = 0;

        for entry in fs::read_dir(sample_dir).expect("read sample directory") {
            let sample_path = entry.expect("read sample entry").path();
            if sample_path.extension().and_then(std::ffi::OsStr::to_str) != Some("json") {
                continue;
            }

            let raw = fs::read_to_string(sample_path).expect("read sample");
            let value: Value = serde_json::from_str(&raw).expect("parse sample");
            let processed = process(&value).expect("process sample");
            let processed_json = serde_json::to_value(processed).expect("serialize processed");

            assert_eq!(processed_json["metadata"]["mail_receiver"], json!("player_71738515"));
            assert_eq!(processed_json["metadata"]["server_id"], json!(1804));
            assert_eq!(processed_json.as_object().map(serde_json::Map::len), Some(1));
            processed_count += 1;
        }

        assert_eq!(processed_count, 5);
    }
}
