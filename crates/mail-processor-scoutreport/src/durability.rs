//! Durability parser for ScoutReport mail.

use mail_processor_sdk::{Extractor, Section};
use serde_json::Value;

use crate::content::{ExtractError, require_child_object, require_content, require_u64_field};

/// Pulls target durability details out of ScoutReport mail content.
#[derive(Debug, Default)]
pub struct DurabilityExtractor;

impl DurabilityExtractor {
    /// Creates a durability extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for DurabilityExtractor {
    fn section(&self) -> &'static str {
        "durability"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let content = require_content(input)?;
        let durability = require_child_object(content, "durability")?;

        let mut section = Section::new();
        section.insert("interval", Value::from(require_u64_field(durability, "interval")?));
        section.insert("max", Value::from(require_u64_field(durability, "max")?));
        section.insert(
            "next_update_time",
            Value::from(require_u64_field(durability, "nextUpdateTime")?),
        );
        section.insert("num", Value::from(require_u64_field(durability, "num")?));
        section.insert("speed", Value::from(require_u64_field(durability, "speed")?));
        section.insert("state", Value::from(require_u64_field(durability, "state")?));
        section
            .insert("state_end_time", Value::from(require_u64_field(durability, "stateEndTime")?));
        Ok(section)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use mail_processor_sdk::Extractor;
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn durability_extractor_reads_fields() {
        let input = json!({
            "body": {
                "content": {
                    "durability": {
                        "interval": 2000,
                        "max": 40000,
                        "nextUpdateTime": 9223372036854775808u64,
                        "num": 30000,
                        "speed": 10,
                        "state": 2,
                        "stateEndTime": 1234
                    }
                }
            }
        });
        let extractor = DurabilityExtractor::new();
        let section = extractor.extract(&input).expect("extract durability");
        let fields = section.fields();

        assert_eq!(fields["interval"], json!(2000));
        assert_eq!(fields["max"], json!(40000));
        assert_eq!(fields["next_update_time"], json!(9223372036854775808u64));
        assert_eq!(fields["num"], json!(30000));
        assert_eq!(fields["speed"], json!(10));
        assert_eq!(fields["state"], json!(2));
        assert_eq!(fields["state_end_time"], json!(1234));
    }

    #[test]
    fn durability_extractor_rejects_missing_field() {
        let input = json!({ "body": { "content": {} } });
        let extractor = DurabilityExtractor::new();
        let err = extractor.extract(&input).expect_err("durability should reject missing field");
        assert!(matches!(err, ExtractError::MissingField { .. }));
    }

    #[test]
    fn roundtrip_durability_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/ScoutReport/Persistent.Mail.136953280177843782931.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = DurabilityExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();

        assert_eq!(fields["interval"], json!(2000));
        assert_eq!(fields["max"], json!(40000));
        assert_eq!(fields["next_update_time"], json!(9223372036854775808u64));
        assert_eq!(fields["num"], json!(40000));
        assert_eq!(fields["speed"], json!(0));
        assert_eq!(fields["state"], json!(1));
        assert_eq!(fields["state_end_time"], json!(0));
    }
}
