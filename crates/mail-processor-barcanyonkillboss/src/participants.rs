//! Participants extractor for BarCanyonKillBoss mail.

use mail_processor_sdk::{ExtractError, Extractor, Section, indexed_array_values};
use serde_json::{Map, Value, json};

use crate::content::{
    require_content, require_number_field, require_string_field, require_u64_field,
};

/// Extracts participant details from BarCanyonKillBoss mail content.
#[derive(Debug, Default)]
pub struct ParticipantsExtractor;

impl ParticipantsExtractor {
    /// Create a new participants extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for ParticipantsExtractor {
    fn section(&self) -> &'static str {
        "participants"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let content = require_content(input)?;
        let infos_value = content
            .get("infos")
            .ok_or(ExtractError::MissingField { field: "infos" })?;
        let infos = indexed_array_values(infos_value, "infos")?;

        let mut participants = Vec::with_capacity(infos.len());
        for info in infos {
            let info = info.as_object().ok_or(ExtractError::InvalidFieldType {
                field: "infos",
                expected: "object",
            })?;
            participants.push(extract_participant(info)?);
        }

        Ok(Section::from_array(participants))
    }
}

fn extract_participant(info: &Map<String, Value>) -> Result<Value, ExtractError> {
    let player_id = require_u64_field(info, "playerId")?;
    let player_name = require_string_field(info, "name")?;
    let damage_rate = require_number_field(info, "damageRate")?;
    let (avatar_url, frame_url) = parse_avatar(info)?;
    let loot = extract_loot(info)?;

    Ok(json!({
        "player_id": player_id,
        "player_name": player_name,
        "avatar_url": avatar_url,
        "frame_url": frame_url,
        "damage_rate": damage_rate,
        "loot": loot,
    }))
}

fn extract_loot(info: &Map<String, Value>) -> Result<Value, ExtractError> {
    let value = info
        .get("loots")
        .ok_or(ExtractError::MissingField { field: "loots" })?;
    let values = indexed_array_values(value, "loots")?;
    let mut loot = Vec::with_capacity(values.len());
    for entry in values {
        let entry = entry.as_object().ok_or(ExtractError::InvalidFieldType {
            field: "loots",
            expected: "object",
        })?;
        let loot_type = require_u64_field(entry, "Type")?;
        let sub_type = require_u64_field(entry, "SubType")?;
        let value = require_u64_field(entry, "Value")?;
        loot.push(json!({
            "type": loot_type,
            "sub_type": sub_type,
            "value": value,
        }));
    }

    Ok(Value::Array(loot))
}

fn parse_avatar(info: &Map<String, Value>) -> Result<(Value, Value), ExtractError> {
    let value = info
        .get("avatar")
        .ok_or(ExtractError::MissingField { field: "avatar" })?;

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
            field: "avatar",
            expected: "string or object",
        }),
    }
}

fn extract_avatar_fields(map: &Map<String, Value>) -> (Value, Value) {
    let avatar_url = map.get("avatar").cloned().unwrap_or(Value::Null);
    let frame_url = map.get("avatarFrame").cloned().unwrap_or(Value::Null);
    (
        normalize_avatar_value(avatar_url),
        normalize_avatar_value(frame_url),
    )
}

fn normalize_avatar_value(value: Value) -> Value {
    match value {
        Value::String(text) if text == "null" => Value::Null,
        other => other,
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
    fn participants_extractor_reads_fields() {
        let input = json!({
            "body": {
                "content": {
                    "infos": [
                        1,
                        {
                            "playerId": 42,
                            "name": "Tester",
                            "avatar": "{\"avatar\":\"https://example.com/a.png\",\"avatarFrame\":\"https://example.com/f.png\"}",
                            "damageRate": 12.5,
                            "loots": [
                                1,
                                { "Type": 2, "SubType": 26, "Value": 3 },
                                2,
                                { "Type": 2, "SubType": 65, "Value": 2 }
                            ]
                        }
                    ]
                }
            }
        });
        let extractor = ParticipantsExtractor::new();
        let section = extractor.extract(&input).unwrap();

        let participants = section.array().expect("participants");
        assert_eq!(participants.len(), 1);
        let participant = &participants[0];
        assert_eq!(participant["player_id"], json!(42));
        assert_eq!(participant["player_name"], json!("Tester"));
        assert_eq!(
            participant["avatar_url"],
            json!("https://example.com/a.png")
        );
        assert_eq!(participant["frame_url"], json!("https://example.com/f.png"));
        assert_eq!(participant["damage_rate"], json!(12.5));
        assert_eq!(participant["loot"].as_array().unwrap().len(), 2);
        assert_eq!(
            participant["loot"][0],
            json!({
                "type": 2,
                "sub_type": 26,
                "value": 3
            })
        );
    }

    #[test]
    fn participants_extractor_allows_string_avatar() {
        let input = json!({
            "body": {
                "content": {
                    "infos": [
                        1,
                        {
                            "playerId": 7,
                            "name": "Solo",
                            "avatar": "https://example.com/a.png",
                            "damageRate": 1,
                            "loots": [1, { "Type": 1, "SubType": 2, "Value": 3 }]
                        }
                    ]
                }
            }
        });
        let extractor = ParticipantsExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let participants = section.array().expect("participants");
        let participant = &participants[0];
        assert_eq!(
            participant["avatar_url"],
            json!("https://example.com/a.png")
        );
        assert_eq!(participant["frame_url"], Value::Null);
    }

    #[test]
    fn participants_extractor_rejects_missing_field() {
        let input = json!({
            "body": {
                "content": {
                    "infos": [
                        1,
                        {
                            "name": "Missing",
                            "avatar": null,
                            "damageRate": 1,
                            "loots": []
                        }
                    ]
                }
            }
        });
        let extractor = ParticipantsExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { .. }));
    }

    #[test]
    fn roundtrip_participants_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/BarCanyonKillBoss/Persistent.Mail.21162669176948646831.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = ParticipantsExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let participants = section.array().expect("participants");
        assert_eq!(participants[0]["player_id"], json!(71738515));
        assert_eq!(participants[0]["player_name"], json!("Grigvar"));
        assert_eq!(participants[0]["damage_rate"], json!(8.681111335754395));
        assert_eq!(participants[0]["loot"].as_array().unwrap().len(), 2);
    }
}
