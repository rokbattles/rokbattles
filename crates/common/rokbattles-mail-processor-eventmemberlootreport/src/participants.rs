//! Builds GVE participant rows from `body.content.infos` in input order.
//!
//! Each row requires `playerId`, `name`, `avatar`, and `loots`. Loot entries require
//! unsigned `Type`, `SubType`, and `Value`. Avatar URLs, objects, and JSON-encoded
//! objects are accepted; missing members and literal `"null"` become JSON null.

use rokbattles_mail_sdk::{ExtractError, Extractor, Section, require_array};
use serde_json::{Map, Value, json};

use crate::content::{require_content, require_string_field, require_u64_field};

/// Extracts the `participants` section using the rules in this module.
#[derive(Debug, Default)]
pub struct ParticipantsExtractor;

impl ParticipantsExtractor {
    /// Creates a stateless extractor.
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
        let infos = content.get("infos").ok_or(ExtractError::MissingField { field: "infos" })?;
        let infos = require_array(infos, "infos")?;
        let participants = infos
            .iter()
            .map(|value| {
                let info = value
                    .as_object()
                    .ok_or(ExtractError::InvalidFieldType { field: "infos", expected: "object" })?;
                extract_participant(info)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Section::from_array(participants))
    }
}

fn extract_participant(info: &Map<String, Value>) -> Result<Value, ExtractError> {
    let player_id = require_u64_field(info, "playerId")?;
    let player_name = require_string_field(info, "name")?;
    let (avatar_url, frame_url) = parse_avatar(info)?;
    let loot = extract_loot(info)?;
    Ok(json!({
        "player_id": player_id,
        "player_name": player_name,
        "avatar_url": avatar_url,
        "frame_url": frame_url,
        "loot": loot,
    }))
}

fn extract_loot(info: &Map<String, Value>) -> Result<Vec<Value>, ExtractError> {
    let loots = info.get("loots").ok_or(ExtractError::MissingField { field: "loots" })?;
    require_array(loots, "loots")?
        .iter()
        .map(|value| {
            let loot = value
                .as_object()
                .ok_or(ExtractError::InvalidFieldType { field: "loots", expected: "object" })?;
            Ok(json!({
                "type": require_u64_field(loot, "Type")?,
                "sub_type": require_u64_field(loot, "SubType")?,
                "value": require_u64_field(loot, "Value")?,
            }))
        })
        .collect()
}

// Avatar strings can hold either a URL or an encoded object. Failed object
// parsing falls back to the original string rather than rejecting a plain URL.
fn parse_avatar(info: &Map<String, Value>) -> Result<(Value, Value), ExtractError> {
    let avatar = info.get("avatar").ok_or(ExtractError::MissingField { field: "avatar" })?;
    match avatar {
        Value::String(text) if text == "null" => Ok((Value::Null, Value::Null)),
        Value::String(text) => match serde_json::from_str::<Value>(text) {
            Ok(Value::Object(map)) => Ok(avatar_fields(&map)),
            _ => Ok((Value::String(text.clone()), Value::Null)),
        },
        Value::Object(map) => Ok(avatar_fields(map)),
        Value::Null => Ok((Value::Null, Value::Null)),
        _ => Err(ExtractError::InvalidFieldType {
            field: "avatar",
            expected: "string, object, or null",
        }),
    }
}

fn avatar_fields(map: &Map<String, Value>) -> (Value, Value) {
    let normalize = |value: Option<&Value>| match value {
        Some(Value::String(text)) if text == "null" => Value::Null,
        Some(value) => value.clone(),
        None => Value::Null,
    };
    (normalize(map.get("avatar")), normalize(map.get("avatarFrame")))
}
