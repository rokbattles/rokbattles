//! Shared player extraction helpers for Battle mail.

use mail_processor_sdk::{ExtractError, indexed_array_values};
use serde_json::{Map, Value, json};

use crate::content::{require_child_object, require_string_field, require_u64_field};

// AppUid app_id prefixes:
// - 2104267: international client
// - 3724753: chinese client
// - 6626468: vietnamese client (gamota)
// - 8518744: korean client
// - 8529460: japanese client
// - 9105753: chinese client (huawei)
// - 9602340: chinese client (tw)
const APP_ID_INTERNATIONAL: u64 = 2_104_267;

/// Extract the common player fields from a Battle character object.
pub(crate) fn extract_player_fields(
    player: &Map<String, Value>,
) -> Result<Map<String, Value>, ExtractError> {
    let player_id = require_signed_id_field(player, "PId")?;
    let player_name = require_string_field(player, "PName")?;
    let kingdom_id = extract_kingdom_id(player)?;
    let alliance_id = require_u64_field(player, "AId")?;
    let alliance_name = require_string_field(player, "AName")?;
    let alliance_abbr = require_string_field(player, "Abbr")?;
    // Alliance building id (AbT) mappings:
    // - 1: flag
    // - 3: stronghold
    // - 11: horse fort (troy kvk)
    let alliance_building_id = optional_u64_field(player, "AbT")?;
    let castle_pos = require_child_object(player, "CastlePos")?;
    let castle_x = require_number_value(castle_pos, "X")?;
    let castle_y = require_number_value(castle_pos, "Y")?;
    let castle_level = require_u64_field(player, "CastleLevel")?;
    let watchtower = optional_u64_field(player, "GtLevel")?;
    // Older battle reports omit CTK entirely; treat it as an empty tracking key.
    let tracking_key = optional_string_field(player, "CTK")?.unwrap_or_default();
    let camp_id = optional_u64_field(player, "SideId")?;
    let rally = optional_bool_field(player, "IsRally")?;
    // Structure id (ShId) mappings:
    // - 22: shrine of war (Ark)
    // - 23: shrine of life (Ark)
    // - 24: sky altar (Ark)
    // - 25: ? (Ark)
    // - 26: outpost of iset (Ark)
    // - 38: Great Ziggurat (KVK)
    // - 51: Lvl 7 Pass (KVK)
    // - 109: obelisk (Ark)
    let structure_id = optional_u64_field(player, "ShId")?;
    let commanders = extract_commanders(player)?;
    let (app_id, app_uid) = extract_app_identity(player)?;
    let (avatar_url, frame_url) = parse_avatar(player)?;
    let supreme_strife = extract_supreme_strife(player)?;

    let mut fields = Map::new();
    fields.insert("player_id".to_string(), Value::from(player_id));
    fields.insert("player_name".to_string(), Value::String(player_name));
    fields.insert(
        "kingdom_id".to_string(),
        kingdom_id.map(Value::from).unwrap_or(Value::Null),
    );
    fields.insert(
        "alliance".to_string(),
        json!({
            "id": alliance_id,
            "name": alliance_name,
            "abbreviation": alliance_abbr,
        }),
    );
    fields.insert(
        "alliance_building_id".to_string(),
        alliance_building_id.map(Value::from).unwrap_or(Value::Null),
    );
    fields.insert(
        "castle".to_string(),
        json!({
            "x": castle_x,
            "y": castle_y,
            "level": castle_level,
            "watchtower": watchtower.map(Value::from).unwrap_or(Value::Null),
        }),
    );
    fields.insert("tracking_key".to_string(), Value::String(tracking_key));
    fields.insert(
        "camp_id".to_string(),
        camp_id.map(Value::from).unwrap_or(Value::Null),
    );
    fields.insert(
        "rally".to_string(),
        rally.map(Value::from).unwrap_or(Value::Null),
    );
    fields.insert(
        "structure_id".to_string(),
        structure_id.map(Value::from).unwrap_or(Value::Null),
    );
    fields.insert("commanders".to_string(), commanders);
    fields.insert(
        "app_id".to_string(),
        app_id.map(Value::from).unwrap_or(Value::Null),
    );
    fields.insert(
        "app_uid".to_string(),
        app_uid.map(Value::from).unwrap_or(Value::Null),
    );
    fields.insert("avatar_url".to_string(), avatar_url);
    fields.insert("frame_url".to_string(), frame_url);
    fields.insert("supreme_strife".to_string(), supreme_strife);
    Ok(fields)
}

/// Extract kingdom id from `COSId`.
pub(crate) fn extract_kingdom_id(player: &Map<String, Value>) -> Result<Option<u64>, ExtractError> {
    optional_u64_field(player, "COSId")
}

/// Require a numeric identifier that can be either signed or unsigned.
fn require_signed_id_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<i64, ExtractError> {
    let value = object
        .get(field)
        .ok_or(ExtractError::MissingField { field })?;
    if let Some(id) = value.as_i64() {
        return Ok(id);
    }
    if let Some(id) = value.as_u64() {
        return i64::try_from(id).map_err(|_| ExtractError::InvalidFieldType {
            field,
            expected: "signed 64-bit integer",
        });
    }
    Err(ExtractError::InvalidFieldType {
        field,
        expected: "integer",
    })
}

fn require_number_value(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Value, ExtractError> {
    let value = object
        .get(field)
        .ok_or(ExtractError::MissingField { field })?;
    if value.is_number() {
        Ok(value.clone())
    } else {
        Err(ExtractError::InvalidFieldType {
            field,
            expected: "number",
        })
    }
}

fn optional_u64_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Option<u64>, ExtractError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .ok_or(ExtractError::InvalidFieldType {
                field,
                expected: "unsigned integer",
            })
            .map(Some),
    }
}

/// Extract the app_id and app_uid values from the AppUid field.
fn extract_app_identity(
    player: &Map<String, Value>,
) -> Result<(Option<u64>, Option<u64>), ExtractError> {
    let app_uid = read_app_uid(player)?;
    match app_uid {
        Some(text) => {
            if let Some((prefix, uid)) = text.split_once('-') {
                let app_id = parse_app_uid_number(prefix, "dash-delimited app_id-app_uid")?;
                let app_uid = parse_app_uid_number(uid, "dash-delimited app_id-app_uid")?;
                Ok((Some(app_id), Some(app_uid)))
            } else {
                let app_uid = parse_app_uid_number(&text, "unsigned integer")?;
                Ok((Some(APP_ID_INTERNATIONAL), Some(app_uid)))
            }
        }
        None => Ok((None, None)),
    }
}

fn parse_app_uid_number(value: &str, expected: &'static str) -> Result<u64, ExtractError> {
    value
        .parse::<u64>()
        .map_err(|_| ExtractError::InvalidFieldType {
            field: "AppUid",
            expected,
        })
}

/// Read the AppUid as a string when present.
fn read_app_uid(player: &Map<String, Value>) -> Result<Option<String>, ExtractError> {
    match player.get("AppUid") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed.to_string()))
            }
        }
        Some(Value::Number(number)) => number.as_u64().map_or_else(
            || {
                Err(ExtractError::InvalidFieldType {
                    field: "AppUid",
                    expected: "string or unsigned integer",
                })
            },
            |value| Ok(Some(value.to_string())),
        ),
        _ => Err(ExtractError::InvalidFieldType {
            field: "AppUid",
            expected: "string or unsigned integer",
        }),
    }
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

fn optional_string_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Option<String>, ExtractError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        _ => Err(ExtractError::InvalidFieldType {
            field,
            expected: "string",
        }),
    }
}

/// Extract Supreme Strife (Titan) details for the player.
fn extract_supreme_strife(player: &Map<String, Value>) -> Result<Value, ExtractError> {
    let value = match player.get("Titan") {
        None | Some(Value::Null) => return Ok(null_supreme_strife()),
        Some(value) => value,
    };
    let titan = value.as_object().ok_or(ExtractError::InvalidFieldType {
        field: "Titan",
        expected: "object",
    })?;
    let battle_id = optional_string_field(titan, "BattleId")?;
    let team_id = optional_u64_field(titan, "TeamId")?;
    let round = optional_u64_field(titan, "Round")?;

    Ok(json!({
        "battle_id": battle_id.map(Value::String).unwrap_or(Value::Null),
        "team_id": team_id.map(Value::from).unwrap_or(Value::Null),
        "round": round.map(Value::from).unwrap_or(Value::Null),
    }))
}

/// Build a null-filled Supreme Strife entry when data is missing.
fn null_supreme_strife() -> Value {
    json!({
        "battle_id": Value::Null,
        "team_id": Value::Null,
        "round": Value::Null,
    })
}

fn extract_commanders(player: &Map<String, Value>) -> Result<Value, ExtractError> {
    let primary = extract_commander(player, &CommanderFieldSet::PRIMARY)?;
    let secondary = extract_commander(player, &CommanderFieldSet::SECONDARY)?;
    Ok(json!({ "primary": primary, "secondary": secondary }))
}

struct CommanderFieldSet {
    id: &'static str,
    level: &'static str,
    formation: &'static str,
    awakened: &'static str,
    star: &'static str,
    equipment: &'static str,
    skills: &'static str,
    relics: &'static str,
    armaments: Option<&'static str>,
}

impl CommanderFieldSet {
    const PRIMARY: CommanderFieldSet = CommanderFieldSet {
        id: "HId",
        level: "HLv",
        formation: "HFMs",
        awakened: "HAw",
        star: "HSt",
        equipment: "HEq",
        skills: "HSS",
        relics: "HClt",
        armaments: Some("HWBs"),
    };
    const SECONDARY: CommanderFieldSet = CommanderFieldSet {
        id: "HId2",
        level: "HLv2",
        formation: "HFMs2",
        awakened: "HAw2",
        star: "HSt2",
        equipment: "HEq2",
        skills: "HSS2",
        relics: "HClt2",
        armaments: None,
    };
}

fn extract_commander(
    player: &Map<String, Value>,
    fields: &CommanderFieldSet,
) -> Result<Value, ExtractError> {
    let id = optional_u64_field(player, fields.id)?;
    let level = optional_u64_field(player, fields.level)?;
    let formation = optional_u64_field(player, fields.formation)?;
    let awakened = optional_bool_field(player, fields.awakened)?;
    let star_level = optional_u64_field(player, fields.star)?;
    let equipment = optional_string_field(player, fields.equipment)?;
    let skills = optional_skills_field(player, fields.skills)?;
    let relics = optional_relics_field(player, fields.relics)?;
    let armaments = match fields.armaments {
        Some(field) => optional_armaments_field(player, field)?,
        None => Value::Null,
    };

    Ok(json!({
        "id": id,
        "level": level,
        "formation": formation,
        "awakened": awakened,
        "star_level": star_level,
        "equipment": equipment,
        "skills": skills,
        "relics": relics,
        "armaments": armaments,
    }))
}

fn optional_skills_field(
    player: &Map<String, Value>,
    field: &'static str,
) -> Result<Value, ExtractError> {
    let value = match player.get(field) {
        None | Some(Value::Null) => return Ok(Value::Null),
        Some(value) => value,
    };

    let values = indexed_array_values(value, field)?;
    let mut skills = Vec::with_capacity(values.len());
    for skill in values {
        let skill = skill.as_object().ok_or(ExtractError::InvalidFieldType {
            field,
            expected: "object",
        })?;
        let skill_id = require_u64_field(skill, "SkillId")?;
        let skill_level = require_u64_field(skill, "SkillLevel")?;
        skills.push(json!({ "id": skill_id, "level": skill_level }));
    }

    Ok(Value::Array(skills))
}

fn optional_relics_field(
    player: &Map<String, Value>,
    field: &'static str,
) -> Result<Value, ExtractError> {
    let value = match player.get(field) {
        None | Some(Value::Null) => return Ok(Value::Null),
        Some(value) => value,
    };

    let values = indexed_array_values(value, field)?;
    if values.len() % 2 != 0 {
        // Some older reports include a single relic id without a level (unlocked but not leveled).
        // Drop the relic list instead of failing or guessing a level.
        return Ok(Value::Null);
    }

    let mut relics = Vec::with_capacity(values.len() / 2);
    for chunk in values.chunks(2) {
        let id = chunk[0].as_u64().ok_or(ExtractError::InvalidFieldType {
            field,
            expected: "unsigned integer",
        })?;
        let level = chunk[1].as_u64().ok_or(ExtractError::InvalidFieldType {
            field,
            expected: "unsigned integer",
        })?;
        relics.push(json!({ "id": id, "level": level }));
    }

    Ok(Value::Array(relics))
}

fn optional_armaments_field(
    player: &Map<String, Value>,
    field: &'static str,
) -> Result<Value, ExtractError> {
    let value = match player.get(field) {
        None | Some(Value::Null) => return Ok(Value::Null),
        Some(value) => value,
    };
    let map = value.as_object().ok_or(ExtractError::InvalidFieldType {
        field,
        expected: "object",
    })?;

    let mut entries = Vec::with_capacity(map.len());
    for (key, value) in map {
        let id = key
            .parse::<u64>()
            .map_err(|_| ExtractError::InvalidFieldType {
                field,
                expected: "numeric object key",
            })?;
        let value = value.as_object().ok_or(ExtractError::InvalidFieldType {
            field,
            expected: "object",
        })?;
        let affix = require_string_field(value, "Affix")?;
        let buffs = require_string_field(value, "Buffs")?;
        entries.push(json!({ "id": id, "affix": affix, "buffs": buffs }));
    }

    entries.sort_by_key(|entry| entry["id"].as_u64().unwrap_or_default());
    Ok(Value::Array(entries))
}

/// Parse the avatar field into avatar and frame URLs.
pub(crate) fn parse_avatar(player: &Map<String, Value>) -> Result<(Value, Value), ExtractError> {
    let value = player
        .get("Avatar")
        .ok_or(ExtractError::MissingField { field: "Avatar" })?;

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
            field: "Avatar",
            expected: "string or object",
        }),
    }
}

/// Normalize the avatar object payload into avatar and frame values.
fn extract_avatar_fields(map: &Map<String, Value>) -> (Value, Value) {
    let avatar_url = map.get("avatar").cloned().unwrap_or(Value::Null);
    let frame_url = map.get("avatarFrame").cloned().unwrap_or(Value::Null);
    (
        normalize_avatar_value(avatar_url),
        normalize_avatar_value(frame_url),
    )
}

/// Convert explicit string null markers into JSON null values.
fn normalize_avatar_value(value: Value) -> Value {
    match value {
        Value::String(text) if text == "null" => Value::Null,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn avatar_pair(input: Value) -> (Value, Value) {
        let object = input.as_object().expect("player object");
        parse_avatar(object).expect("parse avatar")
    }

    fn base_player() -> Map<String, Value> {
        json!({
            "PId": 42,
            "PName": "Sender",
            "COSId": 99,
            "AId": 77,
            "AName": "Alliance",
            "Abbr": "AL",
            "CastlePos": { "X": 10.5, "Y": 20.25 },
            "CastleLevel": 9,
            "CTK": "tracking-1",
            "Avatar": "https://example.com/avatar.png"
        })
        .as_object()
        .expect("base player")
        .clone()
    }

    #[test]
    fn parse_avatar_accepts_url_string() {
        let input = json!({ "Avatar": "https://example.com/avatar.png" });
        let (avatar_url, frame_url) = avatar_pair(input);
        assert_eq!(avatar_url, json!("https://example.com/avatar.png"));
        assert_eq!(frame_url, Value::Null);
    }

    #[test]
    fn parse_avatar_accepts_json_string() {
        let input = json!({
            "Avatar": "{\"avatarFrame\":\"https://example.com/frame.png\",\"avatar\":\"https://example.com/avatar.png\"}"
        });
        let (avatar_url, frame_url) = avatar_pair(input);
        assert_eq!(avatar_url, json!("https://example.com/avatar.png"));
        assert_eq!(frame_url, json!("https://example.com/frame.png"));
    }

    #[test]
    fn parse_avatar_accepts_object() {
        let input = json!({
            "Avatar": {
                "avatar": "https://example.com/avatar.png",
                "avatarFrame": null
            }
        });
        let (avatar_url, frame_url) = avatar_pair(input);
        assert_eq!(avatar_url, json!("https://example.com/avatar.png"));
        assert_eq!(frame_url, Value::Null);
    }

    #[test]
    fn extract_player_fields_reads_identity() {
        let mut player = base_player();
        player.insert("AbT".to_string(), json!(3));
        let fields = extract_player_fields(&player).unwrap();
        assert_eq!(fields.get("player_id"), Some(&json!(42)));
        assert_eq!(fields.get("player_name"), Some(&json!("Sender")));
        assert_eq!(fields.get("kingdom_id"), Some(&json!(99)));
        assert_eq!(
            fields.get("alliance"),
            Some(&json!({ "id": 77, "name": "Alliance", "abbreviation": "AL" }))
        );
        assert_eq!(fields.get("alliance_building_id"), Some(&json!(3)));
        assert_eq!(
            fields.get("avatar_url"),
            Some(&json!("https://example.com/avatar.png"))
        );
    }

    #[test]
    fn extract_player_fields_allows_missing_kingdom_id() {
        let mut player = base_player();
        player.remove("COSId");
        let fields = extract_player_fields(&player).unwrap();
        assert_eq!(fields.get("kingdom_id"), Some(&json!(null)));
    }

    #[test]
    fn extract_player_fields_reads_supreme_strife() {
        let mut player = base_player();
        player.insert(
            "Titan".to_string(),
            json!({ "BattleId": "battle-1", "TeamId": 12, "Round": 3 }),
        );
        let fields = extract_player_fields(&player).unwrap();
        assert_eq!(
            fields.get("supreme_strife"),
            Some(&json!({
                "battle_id": "battle-1",
                "team_id": 12,
                "round": 3
            }))
        );
    }

    #[test]
    fn extract_player_fields_defaults_supreme_strife() {
        let player = base_player();
        let fields = extract_player_fields(&player).unwrap();
        assert_eq!(
            fields.get("supreme_strife"),
            Some(&json!({
                "battle_id": null,
                "team_id": null,
                "round": null
            }))
        );
    }

    #[test]
    fn extract_player_fields_keeps_empty_supreme_strife_battle_id() {
        let mut player = base_player();
        player.insert(
            "Titan".to_string(),
            json!({ "BattleId": "", "TeamId": 0, "Round": 0 }),
        );
        let fields = extract_player_fields(&player).unwrap();
        assert_eq!(
            fields.get("supreme_strife"),
            Some(&json!({
                "battle_id": "",
                "team_id": 0,
                "round": 0
            }))
        );
    }

    #[test]
    fn extract_player_fields_reads_location_fields() {
        let mut player = base_player();
        player.insert("GtLevel".to_string(), json!(12));
        player.insert("SideId".to_string(), json!(3));
        player.insert("IsRally".to_string(), json!(true));
        player.insert("ShId".to_string(), json!(109));
        let fields = extract_player_fields(&player).unwrap();
        assert_eq!(
            fields.get("castle"),
            Some(&json!({
                "x": 10.5,
                "y": 20.25,
                "level": 9,
                "watchtower": 12
            }))
        );
        assert_eq!(fields.get("tracking_key"), Some(&json!("tracking-1")));
        assert_eq!(fields.get("camp_id"), Some(&json!(3)));
        assert_eq!(fields.get("rally"), Some(&json!(true)));
        assert_eq!(fields.get("structure_id"), Some(&json!(109)));
    }

    #[test]
    fn extract_player_fields_defaults_missing_tracking_key() {
        let mut player = base_player();
        player.remove("CTK");
        let fields = extract_player_fields(&player).unwrap();
        assert_eq!(fields.get("tracking_key"), Some(&json!("")));
    }

    #[test]
    fn extract_player_fields_defaults_null_tracking_key() {
        let mut player = base_player();
        player.insert("CTK".to_string(), Value::Null);
        let fields = extract_player_fields(&player).unwrap();
        assert_eq!(fields.get("tracking_key"), Some(&json!("")));
    }

    #[test]
    fn extract_player_fields_ignores_incomplete_secondary_relics() {
        let mut player = base_player();
        player.insert("HClt2".to_string(), json!([1, 6]));
        let fields = extract_player_fields(&player).unwrap();
        assert_eq!(fields["commanders"]["secondary"]["relics"], json!(null));
    }

    #[test]
    fn extract_player_fields_ignores_incomplete_primary_relics() {
        let mut player = base_player();
        player.insert("HClt".to_string(), json!([10001]));
        let fields = extract_player_fields(&player).unwrap();
        assert_eq!(fields["commanders"]["primary"]["relics"], json!(null));
    }

    #[test]
    fn extract_player_fields_reads_commanders() {
        let mut player = base_player();
        player.insert("HId".to_string(), json!(501));
        player.insert("HLv".to_string(), json!(10));
        player.insert("HFMs".to_string(), json!(2));
        player.insert("HAw".to_string(), json!(true));
        player.insert("HSt".to_string(), json!(4));
        player.insert("HEq".to_string(), json!("{1:200}"));
        player.insert(
            "HSS".to_string(),
            json!([1, { "SkillId": 111, "SkillLevel": 3 }]),
        );
        player.insert("HClt".to_string(), json!([1, 10001, 2, 2]));
        player.insert(
            "HWBs".to_string(),
            json!({ "1": { "Affix": "-1", "Buffs": "buffs-1", "Id": "ignored" } }),
        );
        player.insert("HId2".to_string(), json!(502));
        player.insert("HLv2".to_string(), json!(12));
        player.insert("HFMs2".to_string(), json!(4));
        player.insert("HAw2".to_string(), json!(false));
        player.insert("HSt2".to_string(), json!(5));
        player.insert("HEq2".to_string(), json!("{2:201}"));
        player.insert(
            "HSS2".to_string(),
            json!([1, { "SkillId": 222, "SkillLevel": 5 }]),
        );
        player.insert("HClt2".to_string(), json!([1, 20001, 2, 5]));
        let fields = extract_player_fields(&player).unwrap();
        assert_eq!(
            fields.get("commanders"),
            Some(&json!({
                "primary": {
                    "id": 501,
                    "level": 10,
                    "formation": 2,
                    "awakened": true,
                    "star_level": 4,
                    "equipment": "{1:200}",
                    "skills": [{ "id": 111, "level": 3 }],
                    "relics": [{ "id": 10001, "level": 2 }],
                    "armaments": [{ "id": 1, "affix": "-1", "buffs": "buffs-1" }]
                },
                "secondary": {
                    "id": 502,
                    "level": 12,
                    "formation": 4,
                    "awakened": false,
                    "star_level": 5,
                    "equipment": "{2:201}",
                    "skills": [{ "id": 222, "level": 5 }],
                    "relics": [{ "id": 20001, "level": 5 }],
                    "armaments": null
                }
            }))
        );
    }

    #[test]
    fn extract_player_fields_defaults_app_id() {
        let mut player = base_player();
        player.insert("AppUid".to_string(), json!("103134073"));
        let fields = extract_player_fields(&player).unwrap();
        assert_eq!(fields.get("app_id"), Some(&json!(APP_ID_INTERNATIONAL)));
        assert_eq!(fields.get("app_uid"), Some(&json!(103134073)));
    }

    #[test]
    fn extract_player_fields_allows_empty_app_uid() {
        let mut player = base_player();
        player.insert("AppUid".to_string(), json!(""));
        let fields = extract_player_fields(&player).unwrap();
        assert_eq!(fields.get("app_id"), Some(&json!(null)));
        assert_eq!(fields.get("app_uid"), Some(&json!(null)));
    }

    #[test]
    fn extract_player_fields_accepts_negative_id() {
        let input = json!({
            "PId": -2,
            "PName": "Neutral",
            "COSId": 0,
            "AId": 0,
            "AName": "",
            "Abbr": "",
            "CastlePos": { "X": 1, "Y": 2 },
            "CastleLevel": 1,
            "CTK": "",
            "Avatar": null
        });
        let fields = extract_player_fields(input.as_object().expect("player")).unwrap();
        assert_eq!(fields.get("player_id"), Some(&json!(-2)));
        assert_eq!(
            fields.get("alliance"),
            Some(&json!({ "id": 0, "name": "", "abbreviation": "" }))
        );
        assert_eq!(fields.get("alliance_building_id"), Some(&json!(null)));
        assert_eq!(
            fields.get("castle"),
            Some(&json!({
                "x": 1,
                "y": 2,
                "level": 1,
                "watchtower": null
            }))
        );
        assert_eq!(fields.get("tracking_key"), Some(&json!("")));
        assert_eq!(fields.get("camp_id"), Some(&json!(null)));
        assert_eq!(fields.get("rally"), Some(&json!(null)));
        assert_eq!(fields.get("structure_id"), Some(&json!(null)));
        assert_eq!(
            fields.get("commanders"),
            Some(&json!({
                "primary": {
                    "id": null,
                    "level": null,
                    "formation": null,
                    "awakened": null,
                    "star_level": null,
                    "equipment": null,
                    "skills": null,
                    "relics": null,
                    "armaments": null
                },
                "secondary": {
                    "id": null,
                    "level": null,
                    "formation": null,
                    "awakened": null,
                    "star_level": null,
                    "equipment": null,
                    "skills": null,
                    "relics": null,
                    "armaments": null
                }
            }))
        );
        assert_eq!(fields.get("app_id"), Some(&json!(null)));
        assert_eq!(fields.get("app_uid"), Some(&json!(null)));
    }

    #[test]
    fn extract_player_fields_splits_app_uid_prefix() {
        let mut player = base_player();
        player.insert("AppUid".to_string(), json!("8518744-399975"));
        let fields = extract_player_fields(&player).unwrap();
        assert_eq!(fields.get("app_id"), Some(&json!(8518744)));
        assert_eq!(fields.get("app_uid"), Some(&json!(399975)));
    }
}
