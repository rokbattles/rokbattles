#![forbid(unsafe_code)]

//! Parses SystemKaharTreasure mail reports.

mod loot;
mod metadata;

pub use mail_processor_sdk::{ExtractError, Section};
use mail_processor_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Runs the SystemKaharTreasure parser.
pub fn process(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process(input)
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(loot::LootExtractor::new()),
    ])
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn process_roundtrip_extracts_kahar_treasure_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/System/Persistent.Mail.22165348178347040031.json");
        let raw = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&raw).expect("parse sample");

        let processed = process(&value).expect("process sample");
        let processed_json = serde_json::to_value(processed).expect("serialize processed");

        assert_eq!(processed_json["metadata"]["mail_id"], json!("22165348178347040031"));
        assert_eq!(processed_json["metadata"]["mail_receiver"], json!("player_71738515"));
        assert_eq!(processed_json["metadata"]["server_id"], json!(16012));
        assert_eq!(processed_json["metadata"]["mail_time"], json!(1783470400647228u64));
        assert_eq!(processed_json["loot"].as_array().map(Vec::len), Some(5));
        assert_eq!(processed_json["loot"][0], json!({"type": 1, "sub_type": 9, "value": 45000}));
    }
}
