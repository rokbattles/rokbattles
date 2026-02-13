//! Metadata extractor for Battle mail.

use mail_processor_sdk::{ExtractError, Extractor, Section, require_string, require_u64};
use serde_json::{Map, Value};

use crate::content::{require_child_object, require_content, require_string_field};
use crate::player::extract_kingdom_id;

/// Extracts top-level metadata fields from a Battle mail.
#[derive(Debug, Default)]
pub struct MetadataExtractor;

impl MetadataExtractor {
    /// Create a new metadata extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for MetadataExtractor {
    fn section(&self) -> &'static str {
        "metadata"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let mail_id = require_string(input, "id")?;
        let mail_time = require_u64(input, "time")?;
        let mail_receiver = require_string(input, "receiver")?;
        let server_id = require_u64(input, "serverId")?;
        let content = require_content(input)?;
        let mail_role = require_string_field(content, "Role")?;
        let kvk = resolve_kvk(&mail_role, content, server_id)?;

        let mut section = Section::new();
        section.insert("mail_id", Value::String(mail_id));
        section.insert("mail_time", Value::from(mail_time));
        section.insert("mail_receiver", Value::String(mail_receiver));
        section.insert("server_id", Value::from(server_id));
        section.insert("mail_role", Value::String(mail_role));
        section.insert("kvk", Value::Bool(kvk));
        Ok(section)
    }
}

/// Resolve whether the report is from KvK.
///
/// Resolution order:
/// 1. `Role == "dungeon"` always returns `false`.
/// 2. `content.isConquerSeason` when present.
/// 3. `serverId != sender kingdom id` (`COSId`).
fn resolve_kvk(
    mail_role: &str,
    content: &Map<String, Value>,
    server_id: u64,
) -> Result<bool, ExtractError> {
    if mail_role == "dungeon" {
        return Ok(false);
    }

    let sender = require_child_object(content, "SelfChar")?;

    if let Some(value) = optional_bool_field(content, "isConquerSeason")? {
        return Ok(value);
    }

    let kingdom_id = extract_kingdom_id(sender)?;
    Ok(kingdom_id.is_some_and(|id| id != server_id))
}

fn optional_bool_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Option<bool>, ExtractError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_bool()
            .ok_or(ExtractError::InvalidFieldType {
                field,
                expected: "boolean",
            })
            .map(Some),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mail_processor_sdk::Extractor;
    use serde_json::{Value, json};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn metadata_extractor_reads_fields() {
        let input = json!({
            "id": "mail-1",
            "time": 1234,
            "receiver": "player-1",
            "serverId": 55,
            "body": {
                "content": {
                    "Role": "gsmp",
                    "isConquerSeason": true,
                    "SelfChar": {
                        "COSId": 10
                    }
                }
            }
        });
        let extractor = MetadataExtractor::new();
        let section = extractor.extract(&input).unwrap();

        let fields = section.fields();
        assert_eq!(fields["mail_id"], json!("mail-1"));
        assert_eq!(fields["mail_time"], json!(1234));
        assert_eq!(fields["mail_receiver"], json!("player-1"));
        assert_eq!(fields["server_id"], json!(55));
        assert_eq!(fields["mail_role"], json!("gsmp"));
        assert_eq!(fields["kvk"], json!(true));
    }

    #[test]
    fn metadata_extractor_rejects_missing_field() {
        let input = json!({ "id": "mail-1" });
        let extractor = MetadataExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { .. }));
    }

    #[test]
    fn roundtrip_metadata_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Battle/Persistent.Mail.1002579517552941234.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = MetadataExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(fields["mail_id"], json!("1002579517552941234"));
        assert_eq!(fields["mail_receiver"], json!("player_110176153"));
        assert_eq!(fields["server_id"], json!(1804));
        assert_eq!(fields["mail_time"], json!(1755294123041275u64));
        assert_eq!(fields["mail_role"], json!("gsmp"));
        assert_eq!(fields["kvk"], json!(false));
    }

    #[test]
    fn metadata_extractor_uses_content_is_conquer_season_false_when_ids_differ() {
        let input = json!({
            "id": "mail-1",
            "time": 1234,
            "receiver": "player-1",
            "serverId": 55,
            "body": {
                "content": {
                    "Role": "gsmp",
                    "isConquerSeason": false,
                    "SelfChar": {
                        "COSId": 999
                    }
                }
            }
        });
        let extractor = MetadataExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();
        assert_eq!(fields["kvk"], json!(false));
    }

    #[test]
    fn metadata_extractor_falls_back_to_server_and_kingdom_mismatch() {
        let input = json!({
            "id": "mail-1",
            "time": 1234,
            "receiver": "player-1",
            "serverId": 55,
            "body": {
                "content": {
                    "Role": "gsmp",
                    "SelfChar": {
                        "COSId": 999
                    }
                }
            }
        });
        let extractor = MetadataExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();
        assert_eq!(fields["kvk"], json!(true));
    }

    #[test]
    fn metadata_extractor_falls_back_to_server_and_kingdom_match() {
        let input = json!({
            "id": "mail-1",
            "time": 1234,
            "receiver": "player-1",
            "serverId": 55,
            "body": {
                "content": {
                    "Role": "gsmp",
                    "SelfChar": {
                        "COSId": 55
                    }
                }
            }
        });
        let extractor = MetadataExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();
        assert_eq!(fields["kvk"], json!(false));
    }

    #[test]
    fn metadata_extractor_prioritizes_dungeon_role_for_kvk() {
        let input = json!({
            "id": "mail-1",
            "time": 1234,
            "receiver": "player-1",
            "serverId": 55,
            "body": {
                "content": {
                    "Role": "dungeon",
                    "isConquerSeason": true,
                    "SelfChar": {
                        "COSId": 999
                    }
                }
            }
        });
        let extractor = MetadataExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();
        assert_eq!(fields["kvk"], json!(false));
    }
}
