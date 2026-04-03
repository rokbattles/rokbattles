//! Body parser for AllianceAOOIndividualResults mail.

use mail_processor_sdk::{ExtractError, Extractor, Section, require_object};
use serde_json::Value;

use crate::content::{optional_u64_field, require_bool_field, require_child_object};

/// Pulls match-level flags from `body.kvs`.
#[derive(Debug, Default)]
pub struct BodyExtractor;

impl BodyExtractor {
    /// Creates a body extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for BodyExtractor {
    fn section(&self) -> &'static str {
        "body"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let root = require_object(input)?;
        let body = require_child_object(root, "body")?;
        let kvs = require_child_object(body, "kvs")?;
        let win = require_bool_field(kvs, "IsWin")?;
        let team = optional_u64_field(kvs, "Idx")?;

        let mut section = Section::new();
        section.insert("win", Value::Bool(win));
        section.insert("team", team);
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
                    "IsWin": true,
                    "Idx": 1
                }
            }
        });
        let extractor = BodyExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();

        assert_eq!(fields["win"], json!(true));
        assert_eq!(fields["team"], json!(1));
    }

    #[test]
    fn body_extractor_allows_missing_idx() {
        let input = json!({
            "body": {
                "kvs": {
                    "IsWin": true
                }
            }
        });
        let extractor = BodyExtractor::new();
        let section = extractor.extract(&input).expect("extract sample");
        let fields = section.fields();
        assert!(fields["team"].is_null());
    }

    #[test]
    fn roundtrip_body_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.102185429177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = BodyExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();

        assert_eq!(fields["win"], json!(true));
        assert_eq!(fields["team"], json!(0));
    }

    #[test]
    fn roundtrip_body_extracts_custom_sample_without_idx() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.6906964177237730831.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = BodyExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();

        assert_eq!(fields["win"], json!(false));
        assert!(fields["team"].is_null());
    }
}
