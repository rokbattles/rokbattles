//! Commander helpers for DuelBattle2 mail.

use mail_processor_sdk::{ExtractError, indexed_array_values};
use serde_json::{Map, Value, json};

use crate::player::{require_bool_field, require_child_object, require_u64_field};

/// Extract the primary and secondary commander data from a player object.
pub(crate) fn extract_player_commanders(
    player: &Map<String, Value>,
) -> Result<(Value, Value), ExtractError> {
    let heroes = require_child_object(player, "Heroes")?;
    let primary = require_child_object(heroes, "MainHero")?;
    let secondary = require_child_object(heroes, "AssistHero")?;

    Ok((extract_commander(primary)?, extract_commander(secondary)?))
}

fn extract_commander(hero: &Map<String, Value>) -> Result<Value, ExtractError> {
    let hero_id = require_u64_field(hero, "HeroId")?;
    let hero_level = require_u64_field(hero, "HeroLevel")?;
    let star_level = require_u64_field(hero, "Star")?;
    let awakened = require_bool_field(hero, "Awaked")?;
    let skills = extract_skills(hero)?;

    Ok(json!({
        "id": hero_id,
        "level": hero_level,
        "star_level": star_level,
        "awakened": awakened,
        "skills": skills,
    }))
}

fn extract_skills(hero: &Map<String, Value>) -> Result<Vec<Value>, ExtractError> {
    let skills_value = hero
        .get("Skills")
        .ok_or(ExtractError::MissingField { field: "Skills" })?;
    let skills = indexed_array_values(skills_value, "Skills")?;

    let mut entries = Vec::with_capacity(skills.len());
    for skill in skills {
        let skill = skill.as_object().ok_or(ExtractError::InvalidFieldType {
            field: "Skills",
            expected: "object",
        })?;
        let skill_id = require_u64_field(skill, "SkillId")?;
        let level = require_u64_field(skill, "Level")?;
        entries.push(json!({ "id": skill_id, "level": level }));
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::locate_player;
    use serde_json::{Value, json};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn extract_player_commanders_reads_fields() {
        let input = json!({
            "Heroes": {
                "MainHero": {
                    "Awaked": true,
                    "HeroId": 10,
                    "HeroLevel": 50,
                    "Skills": [
                        1,
                        { "Level": 3, "SkillId": 101 },
                        2,
                        { "Level": 4, "SkillId": 102 }
                    ],
                    "Star": 5
                },
                "AssistHero": {
                    "Awaked": false,
                    "HeroId": 20,
                    "HeroLevel": 45,
                    "Skills": [1, { "Level": 2, "SkillId": 201 }],
                    "Star": 4
                }
            }
        });
        let player = input.as_object().expect("player object");
        let (primary, secondary) = extract_player_commanders(player).unwrap();

        assert_eq!(primary["id"], json!(10));
        assert_eq!(primary["level"], json!(50));
        assert_eq!(primary["star_level"], json!(5));
        assert_eq!(primary["awakened"], json!(true));
        assert_eq!(
            primary["skills"],
            json!([
                { "id": 101, "level": 3 },
                { "id": 102, "level": 4 }
            ])
        );

        assert_eq!(secondary["id"], json!(20));
        assert_eq!(secondary["level"], json!(45));
        assert_eq!(secondary["star_level"], json!(4));
        assert_eq!(secondary["awakened"], json!(false));
        assert_eq!(secondary["skills"], json!([{ "id": 201, "level": 2 }]));
    }

    #[test]
    fn roundtrip_sender_commanders_extract_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/DuelBattle2/Persistent.Mail.4197312176618249531.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let player = locate_player(&value, "AtkPlayer").expect("locate sender");
        let (primary, secondary) = extract_player_commanders(player).expect("extract commanders");

        assert_eq!(primary["id"], json!(540));
        assert_eq!(primary["level"], json!(60));
        assert_eq!(primary["star_level"], json!(6));
        assert_eq!(primary["awakened"], json!(true));
        assert_eq!(primary["skills"][0], json!({ "id": 389, "level": 5 }));

        assert_eq!(secondary["id"], json!(575));
        assert_eq!(secondary["level"], json!(60));
        assert_eq!(secondary["star_level"], json!(6));
        assert_eq!(secondary["awakened"], json!(true));
        assert_eq!(secondary["skills"][0], json!({ "id": 411, "level": 5 }));
    }

    #[test]
    fn roundtrip_opponent_commanders_extract_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/DuelBattle2/Persistent.Mail.4197312176618249531.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let player = locate_player(&value, "DefPlayer").expect("locate opponent");
        let (primary, secondary) = extract_player_commanders(player).expect("extract commanders");

        assert_eq!(primary["id"], json!(545));
        assert_eq!(primary["level"], json!(60));
        assert_eq!(primary["star_level"], json!(6));
        assert_eq!(primary["awakened"], json!(true));
        assert_eq!(primary["skills"][0], json!({ "id": 393, "level": 5 }));

        assert_eq!(secondary["id"], json!(185));
        assert_eq!(secondary["level"], json!(60));
        assert_eq!(secondary["star_level"], json!(6));
        assert_eq!(secondary["awakened"], json!(true));
        assert_eq!(secondary["skills"][0], json!({ "id": 251, "level": 5 }));
    }
}
