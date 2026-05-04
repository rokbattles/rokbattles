//! Alliance parser for AllianceAOOBattleResults mail.

use mail_processor_sdk::{ExtractError, Extractor, Section, indexed_array_values};
use serde_json::{Value, json};

use crate::content::{require_body_kvs, require_bool_field, require_u64_field};

/// Pulls alliance-level match stats from `body.kvs.asInfos`.
#[derive(Debug, Default)]
pub struct AlliancesExtractor;

impl AlliancesExtractor {
    /// Creates an alliances extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for AlliancesExtractor {
    fn section(&self) -> &'static str {
        "alliances"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let kvs = require_body_kvs(input)?;
        let as_infos = kvs.get("asInfos").ok_or(ExtractError::MissingField { field: "asInfos" })?;
        let as_infos = indexed_array_values(as_infos, "asInfos")?;

        let mut alliances = Vec::with_capacity(as_infos.len());
        for entry in as_infos {
            let entry = entry
                .as_object()
                .ok_or(ExtractError::InvalidFieldType { field: "asInfos", expected: "object" })?;

            let abbreviation = optional_string_field(entry, "Abbr")?;
            let id = require_u64_field(entry, "AllianceId")?;
            let name = optional_string_field(entry, "Name")?;
            let logo = optional_string_field(entry, "Logo")?;
            let members = require_u64_field(entry, "Members")?;
            let members_max = require_u64_field(entry, "MemberMax")?;
            let power = optional_u64_field(entry, "Power")?;
            let score = require_u64_field(entry, "Score")?;
            let server_id = optional_u64_field(entry, "ServerId")?;
            let is_blue = require_bool_field(entry, "IsBlue")?;
            let team = optional_u64_field(entry, "Idx")?;

            alliances.push(json!({
                "alliance": {
                    "abbreviation": abbreviation,
                    "id": id,
                    "name": name,
                    "logo": logo,
                },
                "members": members,
                "members_max": members_max,
                "power": power,
                "score": score,
                "server_id": server_id,
                "is_blue": is_blue,
                "team": team,
            }));
        }

        Ok(Section::from_array(alliances))
    }
}

fn optional_string_field(
    object: &serde_json::Map<String, Value>,
    field: &'static str,
) -> Result<Value, ExtractError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(Value::Null),
        Some(value) => value
            .as_str()
            .map(|text| Value::String(text.to_string()))
            .ok_or(ExtractError::InvalidFieldType { field, expected: "string" }),
    }
}

fn optional_u64_field(
    object: &serde_json::Map<String, Value>,
    field: &'static str,
) -> Result<Value, ExtractError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(Value::Null),
        Some(value) => value
            .as_u64()
            .map(Value::from)
            .ok_or(ExtractError::InvalidFieldType { field, expected: "unsigned integer" }),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use mail_processor_sdk::Extractor;
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn alliances_extractor_reads_fields() {
        let input = json!({
            "body": {
                "kvs": {
                    "asInfos": [
                        1,
                        {
                            "Abbr": "AAA",
                            "AllianceId": 11,
                            "Name": "Alpha",
                            "Logo": "1_1_1",
                            "Members": 29,
                            "MemberMax": 30,
                            "Power": 123,
                            "Score": 456,
                            "ServerId": 1001,
                            "IsBlue": true,
                            "Idx": 0
                        }
                    ]
                }
            }
        });

        let extractor = AlliancesExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let alliances = section.array().expect("alliances");

        assert_eq!(alliances.len(), 1);
        assert_eq!(alliances[0]["alliance"]["abbreviation"], json!("AAA"));
        assert_eq!(alliances[0]["alliance"]["id"], json!(11));
        assert_eq!(alliances[0]["alliance"]["name"], json!("Alpha"));
        assert_eq!(alliances[0]["alliance"]["logo"], json!("1_1_1"));
        assert_eq!(alliances[0]["members"], json!(29));
        assert_eq!(alliances[0]["members_max"], json!(30));
        assert_eq!(alliances[0]["power"], json!(123));
        assert_eq!(alliances[0]["score"], json!(456));
        assert_eq!(alliances[0]["server_id"], json!(1001));
        assert_eq!(alliances[0]["is_blue"], json!(true));
        assert_eq!(alliances[0]["team"], json!(0));
    }

    #[test]
    fn alliances_extractor_rejects_missing_field() {
        let input = json!({
            "body": {
                "kvs": {}
            }
        });
        let extractor = AlliancesExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { field: "asInfos" }));
    }

    #[test]
    fn roundtrip_alliances_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.102185423177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = AlliancesExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let alliances = section.array().expect("alliances");

        assert_eq!(alliances.len(), 2);
        assert_eq!(alliances[0]["alliance"]["id"], json!(7154636));
        assert_eq!(alliances[0]["alliance"]["name"], json!("3560 War Shell B"));
        assert_eq!(alliances[1]["alliance"]["id"], json!(4808188));
        assert_eq!(alliances[1]["alliance"]["name"], json!("魂社会 Squad Zero"));
    }

    #[test]
    fn roundtrip_alliances_extracts_type_14_sparse_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.6906962177237730831.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = AlliancesExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let alliances = section.array().expect("alliances");

        assert_eq!(alliances.len(), 2);
        assert_eq!(alliances[0]["alliance"]["id"], json!(1));
        assert!(alliances[0]["alliance"]["abbreviation"].is_null());
        assert!(alliances[0]["alliance"]["logo"].is_null());
        assert_eq!(alliances[0]["members"], json!(27));
        assert_eq!(alliances[0]["members_max"], json!(30));
        assert!(alliances[0]["power"].is_null());
        assert_eq!(alliances[0]["score"], json!(30532));
        assert!(alliances[0]["server_id"].is_null());
        assert_eq!(alliances[0]["is_blue"], json!(true));
        assert!(alliances[0]["team"].is_null());
    }
}
