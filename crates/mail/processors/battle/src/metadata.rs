//! Metadata parser for Battle mail.

use mail_sdk::{
    ExtractError, Extractor, Section, extract_base_metadata, optional_bool_field,
    optional_u64_field, require_string_field, require_u64_field,
};
use serde_json::{Map, Value};

use crate::{
    content::{require_child_object, require_content},
    player::extract_kingdom_id,
};

/// Pulls top-level metadata out of a Battle mail.
#[derive(Debug, Default)]
pub struct MetadataExtractor;

impl MetadataExtractor {
    /// Creates a metadata extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for MetadataExtractor {
    fn section(&self) -> &'static str {
        "metadata"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let metadata = extract_base_metadata(input)?;
        let content = require_content(input)?;
        let report_id = require_u64_field(content, "Id")?;
        let mail_role = require_string_field(content, "Role")?;
        let kvk = resolve_kvk(&mail_role, content, metadata.server_id)?;
        let ll_script_schema = optional_u64_field(content, "LLScriptSchema")?;

        let mut section = metadata.into_section();
        section.insert("report_id", Value::from(report_id));
        section.insert("mail_role", Value::String(mail_role));
        section.insert("kvk", Value::Bool(kvk));
        section.insert("ll_script_schema", ll_script_schema.map_or(Value::Null, Value::from));
        Ok(section)
    }
}

/// Figures out whether the report came from KvK.
///
/// Checked in this order:
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

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use mail_sdk::Extractor;
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn metadata_extractor_reads_fields() {
        let input = json!({
            "id": "mail-1",
            "time": 1234,
            "receiver": "player-1",
            "serverId": 55,
            "body": {
                "content": {
                    "Id": 18930744,
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
        assert_eq!(fields["report_id"], json!(18930744));
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
            .join("../../../../samples/Battle/Persistent.Mail.1002579517552941234.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = MetadataExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(fields["mail_id"], json!("1002579517552941234"));
        assert_eq!(fields["mail_receiver"], json!("player_110176153"));
        assert_eq!(fields["server_id"], json!(1804));
        assert_eq!(fields["mail_time"], json!(1755294123041275u64));
        assert_eq!(fields["report_id"], json!(5391170));
        assert_eq!(fields["mail_role"], json!("gsmp"));
        assert_eq!(fields["kvk"], json!(false));
    }

    #[test]
    fn metadata_extractor_rejects_missing_report_id() {
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
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { field: "Id" }));
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
                    "Id": 18930744,
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
                    "Id": 18930744,
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
                    "Id": 18930744,
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
                    "Id": 18930744,
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

    #[test]
    fn metadata_extractor_preserves_ll_script_schema() {
        let input = metadata_input_with_ll_script_schema(Some(json!(320)));

        let section = MetadataExtractor::new().extract(&input).unwrap();

        assert_eq!(section.fields()["ll_script_schema"], json!(320));
    }

    #[test]
    fn metadata_extractor_preserves_zero_ll_script_schema() {
        let input = metadata_input_with_ll_script_schema(Some(json!(0)));

        let section = MetadataExtractor::new().extract(&input).unwrap();

        assert_eq!(section.fields()["ll_script_schema"], json!(0));
    }

    #[test]
    fn metadata_extractor_uses_null_when_ll_script_schema_is_absent() {
        let input = metadata_input_with_ll_script_schema(None);

        let section = MetadataExtractor::new().extract(&input).unwrap();

        assert_eq!(section.fields()["ll_script_schema"], Value::Null);
    }

    fn metadata_input_with_ll_script_schema(ll_script_schema: Option<Value>) -> Value {
        let mut input = json!({
            "id": "mail-1",
            "time": 1234,
            "receiver": "player-1",
            "serverId": 55,
            "body": {
                "content": {
                    "Id": 18930744,
                    "Role": "gsmp",
                    "isConquerSeason": true,
                    "SelfChar": {
                        "COSId": 10
                    }
                }
            }
        });

        if let Some(value) = ll_script_schema {
            input["body"]["content"]["LLScriptSchema"] = value;
        }

        input
    }
}
