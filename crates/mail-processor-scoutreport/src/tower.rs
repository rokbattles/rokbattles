//! Tower parser for ScoutReport mail.

use mail_processor_sdk::{Extractor, Section};
use serde_json::Value;

use crate::content::{ExtractError, require_child_object, require_content, require_u64_field};

/// Pulls target tower details out of ScoutReport mail content.
#[derive(Debug, Default)]
pub struct TowerExtractor;

impl TowerExtractor {
    /// Creates a tower extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for TowerExtractor {
    fn section(&self) -> &'static str {
        "tower"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let content = require_content(input)?;
        let tower = require_child_object(content, "tower")?;

        let mut section = Section::new();
        section.insert("hp", Value::from(require_u64_field(tower, "hp")?));
        section.insert("hp_speed", Value::from(require_u64_field(tower, "hpSpeed")?));
        section.insert("interval", Value::from(require_u64_field(tower, "interval")?));
        section.insert("level", Value::from(require_u64_field(tower, "level")?));
        section
            .insert("next_update_time", Value::from(require_u64_field(tower, "nextUpdateTime")?));
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
    fn tower_extractor_reads_fields() {
        let input = json!({
            "body": {
                "content": {
                    "tower": {
                        "hp": 50000,
                        "hpSpeed": 500,
                        "interval": 60000,
                        "level": 25,
                        "nextUpdateTime": 1234
                    }
                }
            }
        });
        let extractor = TowerExtractor::new();
        let section = extractor.extract(&input).expect("extract tower");
        let fields = section.fields();

        assert_eq!(fields["hp"], json!(50000));
        assert_eq!(fields["hp_speed"], json!(500));
        assert_eq!(fields["interval"], json!(60000));
        assert_eq!(fields["level"], json!(25));
        assert_eq!(fields["next_update_time"], json!(1234));
    }

    #[test]
    fn tower_extractor_rejects_missing_field() {
        let input = json!({ "body": { "content": {} } });
        let extractor = TowerExtractor::new();
        let err = extractor.extract(&input).expect_err("tower should reject missing field");
        assert!(matches!(err, ExtractError::MissingField { .. }));
    }

    #[test]
    fn roundtrip_tower_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/ScoutReport/Persistent.Mail.136953280177843782931.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = TowerExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();

        assert_eq!(fields["hp"], json!(50000));
        assert_eq!(fields["hp_speed"], json!(500));
        assert_eq!(fields["interval"], json!(60000));
        assert_eq!(fields["level"], json!(25));
        assert_eq!(fields["next_update_time"], json!(0));
    }
}
