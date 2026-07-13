//! Body parser for AllianceAOOBattleInfo mail.

use mail_sdk::{
    ExtractError, Extractor, Section, require_array, require_bool_field, require_u64_field,
};
use serde_json::{Value, json};

/// Pulls fight schedule rows from `body.kvs.fightlist`.
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
        let root = input.as_object().ok_or(ExtractError::NotObject)?;
        let body = root
            .get("body")
            .and_then(Value::as_object)
            .ok_or(ExtractError::MissingField { field: "body" })?;
        let kvs = body
            .get("kvs")
            .and_then(Value::as_object)
            .ok_or(ExtractError::MissingField { field: "kvs" })?;
        let fightlist =
            kvs.get("fightlist").ok_or(ExtractError::MissingField { field: "fightlist" })?;
        let fightlist = require_array(fightlist, "fightlist")?;

        let mut fights = Vec::with_capacity(fightlist.len());
        for fight in fightlist {
            let fight = fight
                .as_object()
                .ok_or(ExtractError::InvalidFieldType { field: "fightlist", expected: "object" })?;
            let team = require_u64_field(fight, "Idx")?;
            let time = require_u64_field(fight, "Time")?;
            let win = require_bool_field(fight, "Win")?;
            fights.push(json!({
                "team": team,
                "time": time,
                "win": win,
            }));
        }

        let mut section = Section::new();
        section.insert("fights", Value::Array(fights));
        Ok(section)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use mail_sdk::Extractor;
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn body_extractor_reads_fields() {
        let input = json!({
            "body": {
                "kvs": {
                    "fightlist": [
                        {
                            "Idx": 0,
                            "Time": 1771768966683u64,
                            "Win": true
                        },
                        {
                            "Idx": 1,
                            "Time": 1771770000000u64,
                            "Win": false
                        }
                    ]
                }
            }
        });

        let extractor = BodyExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();
        let fights = fields["fights"].as_array().unwrap();

        assert_eq!(fights.len(), 2);
        assert_eq!(fights[0], json!({ "team": 0, "time": 1771768966683u64, "win": true }));
        assert_eq!(fights[1], json!({ "team": 1, "time": 1771770000000u64, "win": false }));
    }

    #[test]
    fn body_extractor_accepts_plain_arrays() {
        let input = json!({
            "body": {
                "kvs": {
                    "fightlist": [
                        {
                            "Idx": 2,
                            "Time": 1771771000000u64,
                            "Win": true
                        }
                    ]
                }
            }
        });

        let extractor = BodyExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();
        let fights = fields["fights"].as_array().unwrap();

        assert_eq!(fights.len(), 1);
        assert_eq!(fights[0], json!({ "team": 2, "time": 1771771000000u64, "win": true }));
    }

    #[test]
    fn body_extractor_rejects_missing_field() {
        let input = json!({
            "body": {
                "kvs": {}
            }
        });
        let extractor = BodyExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { field: "fightlist" }));
    }

    #[test]
    fn roundtrip_body_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../samples/Alliance/Persistent.Mail.102185425177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = BodyExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        let fights = fields["fights"].as_array().expect("fights");

        assert_eq!(fights.len(), 1);
        assert_eq!(
            fights[0],
            json!({
                "team": 0,
                "time": 1771768966683u64,
                "win": true
            })
        );
    }
}
