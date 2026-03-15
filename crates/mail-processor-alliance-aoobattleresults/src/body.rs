//! Body extractor for AllianceAOOBattleResults mail.

use mail_processor_sdk::{ExtractError, Extractor, Section};
use serde_json::{Value, json};

use crate::content::{require_body_kvs, require_bool_field, require_u64_field};

/// Extracts top-level battle result flags from `body.kvs`.
#[derive(Debug, Default)]
pub struct BodyExtractor;

impl BodyExtractor {
    /// Create a new body extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for BodyExtractor {
    fn section(&self) -> &'static str {
        "body"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let kvs = require_body_kvs(input)?;
        let win = require_bool_field(kvs, "isWin")?;
        let alliance_id = require_u64_field(kvs, "myAsId")?;

        let mut section = Section::new();
        section.insert("win", Value::Bool(win));
        section.insert("alliance", json!({ "id": alliance_id }));
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
    fn body_extractor_reads_fields() {
        let input = json!({
            "body": {
                "kvs": {
                    "isWin": true,
                    "myAsId": 42
                }
            }
        });

        let extractor = BodyExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();

        assert_eq!(fields["win"], json!(true));
        assert_eq!(fields["alliance"]["id"], json!(42));
    }

    #[test]
    fn body_extractor_rejects_missing_field() {
        let input = json!({
            "body": {
                "kvs": {
                    "isWin": true
                }
            }
        });

        let extractor = BodyExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { field: "myAsId" }));
    }

    #[test]
    fn roundtrip_body_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.102185423177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = BodyExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();

        assert_eq!(fields["win"], json!(true));
        assert_eq!(fields["alliance"]["id"], json!(4808188));
    }
}
