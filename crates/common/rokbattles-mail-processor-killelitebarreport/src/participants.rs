//! Builds the participant array from `body.content.infos` in input order.
//!
//! Each row requires identity, numeric damage share, an avatar field, and a loot
//! array. Loot fields are renamed without merging duplicate entries. Avatar
//! objects may be embedded as JSON strings; other strings remain avatar values.
//! The literal string `"null"` is normalized both at the root and inside avatars.

use rokbattles_mail_sdk::{ExtractError, Extractor, Section, require_array};
use serde_json::{Map, Value, json};

use crate::content::{
    require_content, require_number_field, require_string_field, require_u64_field,
};

/// Extracts participant details out of KillEliteBarReport mail content.
#[derive(Debug, Default)]
pub struct ParticipantsExtractor;

impl ParticipantsExtractor {
    /// Creates a participants extractor.
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

        let infos_value =
            content.get("infos").ok_or(ExtractError::MissingField { field: "infos" })?;
        let infos = require_array(infos_value, "infos")?;

        let mut participants = Vec::with_capacity(infos.len());

        for info in infos {
            let info = info
                .as_object()
                .ok_or(ExtractError::InvalidFieldType { field: "infos", expected: "object" })?;

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
    let value = info.get("loots").ok_or(ExtractError::MissingField { field: "loots" })?;
    let values = require_array(value, "loots")?;

    let mut loot = Vec::with_capacity(values.len());

    for entry in values {
        let entry = entry
            .as_object()
            .ok_or(ExtractError::InvalidFieldType { field: "loots", expected: "object" })?;

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

// Avatar strings can hold either a URL or an encoded object. Failed object
// parsing falls back to the original string rather than rejecting a plain URL.
fn parse_avatar(info: &Map<String, Value>) -> Result<(Value, Value), ExtractError> {
    let value = info.get("avatar").ok_or(ExtractError::MissingField { field: "avatar" })?;

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
        _ => Err(ExtractError::InvalidFieldType { field: "avatar", expected: "string or object" }),
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
