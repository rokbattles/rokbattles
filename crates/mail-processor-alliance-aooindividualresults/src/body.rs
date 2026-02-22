//! Body extractor for AllianceAOOIndividualResults mail.

use mail_processor_sdk::{ExtractError, Extractor, Section, require_object};
use serde_json::{Map, Value};

/// Extracts match-level flags from `body.kvs`.
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
        let root = require_object(input)?;
        let body = require_child_object(root, "body")?;
        let kvs = require_child_object(body, "kvs")?;
        let win = require_bool_field(kvs, "IsWin")?;
        let team = require_u64_field(kvs, "Idx")?;

        let mut section = Section::new();
        section.insert("win", Value::Bool(win));
        section.insert("team", Value::from(team));
        Ok(section)
    }
}

fn require_child_object<'a>(
    object: &'a Map<String, Value>,
    field: &'static str,
) -> Result<&'a Map<String, Value>, ExtractError> {
    let value = object
        .get(field)
        .ok_or(ExtractError::MissingField { field })?;
    value.as_object().ok_or(ExtractError::InvalidFieldType {
        field,
        expected: "object",
    })
}

fn require_u64_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<u64, ExtractError> {
    let value = object
        .get(field)
        .ok_or(ExtractError::MissingField { field })?;
    value.as_u64().ok_or(ExtractError::InvalidFieldType {
        field,
        expected: "unsigned integer",
    })
}

fn require_bool_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<bool, ExtractError> {
    let value = object
        .get(field)
        .ok_or(ExtractError::MissingField { field })?;
    value.as_bool().ok_or(ExtractError::InvalidFieldType {
        field,
        expected: "boolean",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mail_processor_sdk::Extractor;
    use serde_json::{Value, json};
    use std::fs;
    use std::path::PathBuf;

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
    fn body_extractor_rejects_missing_field() {
        let input = json!({
            "body": {
                "kvs": {
                    "IsWin": true
                }
            }
        });
        let extractor = BodyExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { field: "Idx" }));
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
}
