//! Character parser for ScoutReport mail.

use mail_processor_sdk::{Extractor, Section};
use serde_json::{Map, Value, json};

use crate::content::{
    ExtractError, require_child_object, require_content, require_number_field,
    require_string_field, require_u64_field,
};

/// Pulls target character details out of ScoutReport mail content.
#[derive(Debug, Default)]
pub struct CharacterExtractor;

impl CharacterExtractor {
    /// Creates a character extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for CharacterExtractor {
    fn section(&self) -> &'static str {
        "character"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let content = require_content(input)?;
        // 1 - alliance flag
        // 3 - alliance fortress
        let alliance_building_type = require_u64_field(content, "allianceBuildingType")?;
        let character = require_child_object(content, "char")?;
        let alliance = require_child_object(character, "alliance")?;
        // 3 - city
        // 6 - resource point (food)
        // 7 - alliance building
        let scout_type = require_u64_field(character, "type")?;
        let position = require_child_object(character, "pos")?;
        let position_x = require_number_field(position, "x")?;
        let position_y = require_number_field(position, "y")?;
        let troop = require_child_object(content, "troop")?;
        let player_id = require_u64_field(troop, "playerId")?;
        let player_name = require_string_field(troop, "playerName")?;
        let (avatar_url, frame_url) = parse_player_avatar(troop)?;

        let mut section = Section::new();
        section.insert("alliance_building_type", Value::from(alliance_building_type));
        section.insert("alliance", build_alliance(alliance)?);
        section.insert("scout_type", Value::from(scout_type));
        section.insert("position", json!({ "x": position_x, "y": position_y }));
        section.insert("avatar_url", avatar_url);
        section.insert("frame_url", frame_url);
        section.insert("player_id", Value::from(player_id));
        section.insert("player_name", Value::String(player_name));
        Ok(section)
    }
}

fn build_alliance(alliance: &Map<String, Value>) -> Result<Value, ExtractError> {
    let abbreviation = require_string_field(alliance, "abbr")?;
    let id = require_u64_field(alliance, "id")?;
    let logo = require_string_field(alliance, "logo")?;
    let name = require_string_field(alliance, "name")?;

    Ok(json!({
        "abbreviation": abbreviation,
        "id": id,
        "logo": logo,
        "name": name,
    }))
}

fn parse_player_avatar(troop: &Map<String, Value>) -> Result<(Value, Value), ExtractError> {
    let value =
        troop.get("playerAvatar").ok_or(ExtractError::MissingField { field: "playerAvatar" })?;

    match value {
        Value::String(text) => {
            if text == "null" {
                return Ok((Value::Null, Value::Null));
            }
            match serde_json::from_str::<Value>(text) {
                Ok(Value::Object(map)) => Ok(extract_avatar_fields(&map)),
                _ => Ok((Value::String(text.clone()), Value::Null)),
            }
        }
        Value::Object(map) => Ok(extract_avatar_fields(map)),
        Value::Null => Ok((Value::Null, Value::Null)),
        _ => Err(ExtractError::InvalidFieldType {
            field: "playerAvatar",
            expected: "string or object",
        }),
    }
}

fn extract_avatar_fields(map: &Map<String, Value>) -> (Value, Value) {
    let avatar_url = map.get("avatar").cloned().unwrap_or(Value::Null);
    let frame_url = map.get("avatarFrame").cloned().unwrap_or(Value::Null);
    (normalize_avatar_value(avatar_url), normalize_avatar_value(frame_url))
}

fn normalize_avatar_value(value: Value) -> Value {
    match value {
        Value::String(text) if text == "null" => Value::Null,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use mail_processor_sdk::Extractor;
    use serde_json::{Value, json};

    use super::*;

    fn avatar_pair(input: Value) -> (Value, Value) {
        let object = input.as_object().expect("troop object");
        parse_player_avatar(object).expect("parse avatar")
    }

    #[test]
    fn character_extractor_reads_fields() {
        let input = json!({
            "body": {
                "content": {
                    "allianceBuildingType": 3,
                    "char": {
                        "alliance": {
                            "abbr": "ABCD",
                            "id": 42,
                            "logo": "1_2_3",
                            "name": "Alliance Name"
                        },
                        "pos": { "x": 12.5, "y": 25 },
                        "type": 7
                    },
                    "troop": {
                        "playerAvatar": "{\"avatarFrame\":\"https://example.com/frame.png\",\"avatar\":\"https://example.com/avatar.png\"}",
                        "playerId": 100,
                        "playerName": "Player One"
                    }
                }
            }
        });
        let extractor = CharacterExtractor::new();
        let section = extractor.extract(&input).expect("extract character");

        let fields = section.fields();
        assert_eq!(fields["alliance_building_type"], json!(3));
        assert_eq!(
            fields["alliance"],
            json!({
                "abbreviation": "ABCD",
                "id": 42,
                "logo": "1_2_3",
                "name": "Alliance Name",
            })
        );
        assert_eq!(fields["scout_type"], json!(7));
        assert_eq!(fields["position"], json!({ "x": 12.5, "y": 25 }));
        assert_eq!(fields["avatar_url"], json!("https://example.com/avatar.png"));
        assert_eq!(fields["frame_url"], json!("https://example.com/frame.png"));
        assert_eq!(fields["player_id"], json!(100));
        assert_eq!(fields["player_name"], json!("Player One"));
    }

    #[test]
    fn parse_player_avatar_accepts_url_string() {
        let input = json!({ "playerAvatar": "https://example.com/avatar.png" });
        let (avatar_url, frame_url) = avatar_pair(input);
        assert_eq!(avatar_url, json!("https://example.com/avatar.png"));
        assert_eq!(frame_url, Value::Null);
    }

    #[test]
    fn parse_player_avatar_accepts_json_string_with_null_frame() {
        let input = json!({
            "playerAvatar": "{\"avatarFrame\":\"null\",\"avatar\":\"https://example.com/avatar.png\"}"
        });
        let (avatar_url, frame_url) = avatar_pair(input);
        assert_eq!(avatar_url, json!("https://example.com/avatar.png"));
        assert_eq!(frame_url, Value::Null);
    }

    #[test]
    fn parse_player_avatar_accepts_object() {
        let input = json!({
            "playerAvatar": {
                "avatar": "https://example.com/avatar.png",
                "avatarFrame": null
            }
        });
        let (avatar_url, frame_url) = avatar_pair(input);
        assert_eq!(avatar_url, json!("https://example.com/avatar.png"));
        assert_eq!(frame_url, Value::Null);
    }

    #[test]
    fn character_extractor_rejects_missing_field() {
        let input = json!({
            "body": {
                "content": {
                    "char": {
                        "alliance": {
                            "abbr": "ABCD",
                            "id": 42,
                            "logo": "1_2_3",
                            "name": "Alliance Name"
                        },
                        "pos": { "x": 12.5, "y": 25 },
                        "type": 7
                    },
                    "troop": {
                        "playerAvatar": null,
                        "playerId": 100,
                        "playerName": "Player One"
                    }
                }
            }
        });
        let extractor = CharacterExtractor::new();
        let err = extractor.extract(&input).expect_err("character should reject missing fields");
        assert!(matches!(err, ExtractError::MissingField { .. }));
    }

    #[test]
    fn roundtrip_character_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/ScoutReport/Persistent.Mail.136953280177843782931.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = CharacterExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(fields["alliance_building_type"], json!(0));
        assert_eq!(
            fields["alliance"],
            json!({
                "abbreviation": "SO4L",
                "id": 4805441,
                "logo": "4_3_15_1_0_3",
                "name": "魂社会SoulReapers",
            })
        );
        assert_eq!(fields["scout_type"], json!(3));
        assert_eq!(fields["position"], json!({ "x": 3829.63671875, "y": 3963.79296875 }));
        assert_eq!(
            fields["avatar_url"],
            json!("http://imimg.lilithcdn.com/roc/img_player_head06.png")
        );
        assert_eq!(
            fields["frame_url"],
            json!("http://imimg.lilithcdn.com/roc/img_ProfileBg220x220_a.png")
        );
        assert_eq!(fields["player_id"], json!(37556801));
        assert_eq!(fields["player_name"], json!("Benbu"));
    }
}
