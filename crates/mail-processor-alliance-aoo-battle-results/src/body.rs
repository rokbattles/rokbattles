//! Body parser for AllianceAOOBattleResults mail.

use mail_processor_sdk::{
    ExtractError, Extractor, Section, optional_u64_or_string_field, require_u64_or_string_field,
};
use serde_json::{Value, json};

use crate::content::{
    require_body_kvs, require_bool_field, require_child_object, require_object,
    require_string_field, require_u64_field,
};

/// Pulls top-level battle result flags from `body.kvs`.
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
        let body_type = require_u64_or_string_field(body, "type")?;
        let body_param = optional_u64_or_string_field(body, "param")?;
        let kvs = require_body_kvs(input)?;
        let battle_type = require_string_field(kvs, "BattleType")?;
        let win = require_bool_field(kvs, "isWin")?;
        let alliance_id = require_u64_field(kvs, "myAsId")?;

        let mut section = Section::new();
        section.insert("type", Value::from(body_type));
        section.insert("param", body_param.map_or(Value::Null, Value::from));
        section.insert("battle_type", Value::String(battle_type));
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
                "type": 60,
                "param": 1,
                "kvs": {
                    "BattleType": "Egypt",
                    "isWin": true,
                    "myAsId": 42
                }
            }
        });

        let extractor = BodyExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();

        assert_eq!(fields["type"], json!(60));
        assert_eq!(fields["param"], json!(1));
        assert_eq!(fields["battle_type"], json!("Egypt"));
        assert_eq!(fields["win"], json!(true));
        assert_eq!(fields["alliance"]["id"], json!(42));
    }

    #[test]
    fn body_extractor_rejects_missing_field() {
        let input = json!({
            "body": {
                "type": 60,
                "kvs": {
                    "BattleType": "Egypt",
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

        assert_eq!(fields["type"], json!(60));
        assert_eq!(fields["param"], json!(1));
        assert_eq!(fields["battle_type"], json!("Egypt"));
        assert_eq!(fields["win"], json!(true));
        assert_eq!(fields["alliance"]["id"], json!(4808188));
    }

    #[test]
    fn roundtrip_body_extracts_type_14_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.6906962177237730831.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = BodyExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();

        assert_eq!(fields["type"], json!(14));
        assert_eq!(fields["param"], json!(1));
        assert_eq!(fields["battle_type"], json!("diy_egypt"));
        assert_eq!(fields["win"], json!(false));
    }

    #[test]
    fn roundtrip_body_extracts_type_14_league_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.47482462178023613231.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = BodyExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();

        assert_eq!(fields["battle_type"], json!("EgyptLeague"));
    }
}
