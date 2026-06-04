#![forbid(unsafe_code)]

//! Parses SystemBarbarianFort mail reports.

mod body;
mod content;
mod metadata;
mod rewards;
mod templates;

pub use mail_processor_sdk::{ExtractError, Section};
use mail_processor_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Runs the SystemBarbarianFort parser.
pub fn process(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process(input)
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(rewards::RewardsExtractor::new()),
        Box::new(body::BodyExtractor::new()),
    ])
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn process_roundtrip_extracts_marauder_encampment_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/System/Persistent.Mail.54530305177357763431.json");
        let raw = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&raw).expect("parse sample");

        let processed = process(&value).expect("process sample");
        let processed_json = serde_json::to_value(processed).expect("serialize processed");

        assert_eq!(processed_json["metadata"]["mail_id"], json!("54530305177357763431"));
        assert_eq!(processed_json["metadata"]["mail_receiver"], json!("player_71738515"));
        assert_eq!(processed_json["body"]["target_name"], json!("Level11"));
        assert_eq!(processed_json["body"]["sub_type"], json!(11));
        assert_eq!(processed_json["body"]["sub_param"], json!(3));
        assert_eq!(
            processed_json["body"]["content"],
            json!({ "percentage": 15.0, "tier": 3, "level": 11 })
        );
        assert_eq!(
            processed_json["body"]["pos"],
            json!({ "x": 7033.7001953125, "y": 1246.9722900390625 })
        );
        assert_eq!(processed_json["rewards"].as_array().map(Vec::len), Some(5));
        assert_eq!(processed_json["rewards"][0], json!({"type": 2, "sub_type": 58, "value": 18}));
    }
}
