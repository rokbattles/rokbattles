//! Shared player helpers for DuelBattle2 sections.

pub(crate) use mail_sdk::{
    ExtractError, Section, indexed_array_values, require_bool_field, require_child_object,
    require_number_field, require_object, require_string_field, require_u64_field,
};
use serde_json::{Map, Value, json};

/// Finds a player object under the requested parent field.
pub(crate) fn locate_player<'a>(
    input: &'a Value,
    parent: &'static str,
) -> Result<&'a Map<String, Value>, ExtractError> {
    let root = require_object(input)?;
    let body = require_child_object(root, "body")?;
    let detail = require_child_object(body, "detail")?;
    require_child_object(detail, parent)
}

/// Pulls the common player fields out of a player object.
pub(crate) fn extract_player_section_from_map(
    player: &Map<String, Value>,
) -> Result<Section, ExtractError> {
    let player_id = require_u64_field(player, "PlayerId")?;
    let player_name = require_string_field(player, "PlayerName")?;
    let abbreviation = require_string_field(player, "Abbr")?;
    let duel_team_id = require_u64_field(player, "DuelTeamId")?;
    let (avatar_url, frame_url) = parse_player_avatar(player)?;

    let mut section = Section::new();
    section.insert("player_id", Value::from(player_id));
    section.insert("player_name", Value::String(player_name));
    section.insert("avatar_url", avatar_url);
    section.insert("frame_url", frame_url);
    section.insert("alliance", json!({ "abbreviation": abbreviation }));
    section.insert("duel", json!({ "team_id": duel_team_id }));
    Ok(section)
}

/// Pulls the buff list out of a player object.
pub(crate) fn extract_player_buffs(
    player: &Map<String, Value>,
) -> Result<Vec<Value>, ExtractError> {
    let heroes = require_child_object(player, "Heroes")?;
    let buffs_value = heroes.get("Buffs").ok_or(ExtractError::MissingField { field: "Buffs" })?;
    let buffs = indexed_array_values(buffs_value, "Buffs")?;

    let mut entries = Vec::with_capacity(buffs.len());
    for buff in buffs {
        let buff = buff
            .as_object()
            .ok_or(ExtractError::InvalidFieldType { field: "Buffs", expected: "object" })?;
        let buff_id = require_u64_field(buff, "BuffId")?;
        let buff_value = require_number_field(buff, "BuffValue")?;
        entries.push(json!({ "id": buff_id, "value": buff_value }));
    }

    Ok(entries)
}

fn parse_player_avatar(object: &Map<String, Value>) -> Result<(Value, Value), ExtractError> {
    let value =
        object.get("PlayerAvatar").ok_or(ExtractError::MissingField { field: "PlayerAvatar" })?;

    match value {
        Value::String(url) => match serde_json::from_str::<Value>(url) {
            Ok(Value::Object(map)) => Ok(extract_avatar_fields(&map)),
            _ => Ok((Value::String(url.clone()), Value::Null)),
        },
        Value::Object(map) => Ok(extract_avatar_fields(map)),
        Value::Null => Ok((Value::Null, Value::Null)),
        _ => Err(ExtractError::InvalidFieldType {
            field: "PlayerAvatar",
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

    use serde_json::{Value, json};

    use super::*;

    fn avatar_pair(input: Value) -> (Value, Value) {
        let object = input.as_object().expect("object");
        parse_player_avatar(object).expect("parse avatar")
    }

    #[test]
    fn parse_player_avatar_accepts_url_string() {
        let input = json!({ "PlayerAvatar": "https://example.com/avatar.png" });
        let (avatar_url, frame_url) = avatar_pair(input);
        assert_eq!(avatar_url, json!("https://example.com/avatar.png"));
        assert_eq!(frame_url, Value::Null);
    }

    #[test]
    fn parse_player_avatar_accepts_json_string() {
        let input = json!({
            "PlayerAvatar": "{\"avatarFrame\":\"https://example.com/frame.png\",\"avatar\":\"https://example.com/avatar.png\"}"
        });
        let (avatar_url, frame_url) = avatar_pair(input);
        assert_eq!(avatar_url, json!("https://example.com/avatar.png"));
        assert_eq!(frame_url, json!("https://example.com/frame.png"));
    }

    #[test]
    fn parse_player_avatar_accepts_json_string_with_null_frame() {
        let input = json!({
            "PlayerAvatar": "{\"avatarFrame\":\"null\",\"avatar\":\"https://example.com/avatar.png\"}"
        });
        let (avatar_url, frame_url) = avatar_pair(input);
        assert_eq!(avatar_url, json!("https://example.com/avatar.png"));
        assert_eq!(frame_url, Value::Null);
    }

    #[test]
    fn parse_player_avatar_accepts_object() {
        let input = json!({
            "PlayerAvatar": {
                "avatar": "https://example.com/avatar.png",
                "avatarFrame": null
            }
        });
        let (avatar_url, frame_url) = avatar_pair(input);
        assert_eq!(avatar_url, json!("https://example.com/avatar.png"));
        assert_eq!(frame_url, Value::Null);
    }

    #[test]
    fn extract_player_buffs_reads_entries() {
        let input = json!({
            "Heroes": {
                "Buffs": [
                    1,
                    { "BuffId": 10, "BuffValue": 1.25 },
                    2,
                    { "BuffId": 11, "BuffValue": 2 }
                ]
            }
        });
        let player = input.as_object().expect("player object");
        let buffs = extract_player_buffs(player).unwrap();
        assert_eq!(
            buffs,
            vec![json!({ "id": 10, "value": 1.25 }), json!({ "id": 11, "value": 2 })]
        );
    }

    #[test]
    fn roundtrip_sender_buffs_extract_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/DuelBattle2/Persistent.Mail.4197312176618249531.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let player = locate_player(&value, "AtkPlayer").expect("locate sender");
        let buffs = extract_player_buffs(player).expect("extract buffs");
        assert_eq!(buffs[0], json!({ "id": 5512, "value": 6 }));
    }

    #[test]
    fn roundtrip_opponent_buffs_extract_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/DuelBattle2/Persistent.Mail.4197312176618249531.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let player = locate_player(&value, "DefPlayer").expect("locate opponent");
        let buffs = extract_player_buffs(player).expect("extract buffs");
        assert_eq!(buffs[0], json!({ "id": 20066, "value": 0.01 }));
    }
}
