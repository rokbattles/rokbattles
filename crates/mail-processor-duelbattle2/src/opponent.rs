//! Opponent extractor for DuelBattle2 mail.

use mail_processor_sdk::{ExtractError, Extractor, Section};
use serde_json::Value;

use crate::commander::extract_player_commanders;
use crate::player::{extract_player_buffs, extract_player_section_from_map, locate_player};

/// Extracts opponent details from the defending player data.
#[derive(Debug, Default)]
pub struct OpponentExtractor;

impl OpponentExtractor {
    /// Create a new opponent extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for OpponentExtractor {
    fn section(&self) -> &'static str {
        "opponent"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let player = locate_player(input, "DefPlayer")?;
        let mut section = extract_player_section_from_map(player)?;
        let (primary, secondary) = extract_player_commanders(player)?;
        let buffs = extract_player_buffs(player)?;
        section.insert("primary_commander", primary);
        section.insert("secondary_commander", secondary);
        section.insert("buffs", Value::Array(buffs));
        Ok(section)
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
    fn opponent_extractor_handles_object_avatar() {
        let input = json!({
            "body": {
                "detail": {
                    "DefPlayer": {
                        "PlayerId": 200,
                        "PlayerName": "Defender",
                        "PlayerAvatar": {
                            "avatar": "https://example.com/avatar2.png",
                            "avatarFrame": null
                        },
                        "Abbr": "DEF",
                        "DuelTeamId": 701,
                        "Heroes": {
                            "MainHero": {
                                "Awaked": true,
                                "HeroId": 10,
                                "HeroLevel": 50,
                                "Skills": [1, { "Level": 3, "SkillId": 101 }],
                                "Star": 5
                            },
                            "AssistHero": {
                                "Awaked": false,
                                "HeroId": 20,
                                "HeroLevel": 45,
                                "Skills": [1, { "Level": 2, "SkillId": 201 }],
                                "Star": 4
                            },
                            "Buffs": [
                                1,
                                { "BuffId": 10, "BuffValue": 1.25 },
                                2,
                                { "BuffId": 11, "BuffValue": 2 }
                            ]
                        }
                    }
                }
            }
        });
        let extractor = OpponentExtractor::new();
        let section = extractor.extract(&input).unwrap();

        let fields = section.fields();
        assert_eq!(fields["player_id"], json!(200));
        assert_eq!(fields["player_name"], json!("Defender"));
        assert_eq!(
            fields["avatar_url"],
            json!("https://example.com/avatar2.png")
        );
        assert_eq!(fields["frame_url"], json!(null));
        assert_eq!(fields["alliance"], json!({ "abbreviation": "DEF" }));
        assert_eq!(fields["duel"], json!({ "team_id": 701 }));
        assert_eq!(fields["primary_commander"]["id"], json!(10));
        assert_eq!(fields["secondary_commander"]["id"], json!(20));
        assert_eq!(fields["buffs"][0], json!({ "id": 10, "value": 1.25 }));
    }

    #[test]
    fn roundtrip_opponent_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/DuelBattle2/Persistent.Mail.4197312176618249531.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = OpponentExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(fields["player_id"], json!(51148833));
        assert_eq!(fields["player_name"], json!("EvilGenius666"));
        assert_eq!(
            fields["avatar_url"],
            json!("http://imimg.lilithcdn.com/roc/img_player_head_festive_1016.png")
        );
        assert_eq!(fields["frame_url"], json!(null));
        assert_eq!(fields["alliance"], json!({ "abbreviation": "HELL" }));
        assert_eq!(fields["duel"], json!({ "team_id": 1747157902 }));
        assert_eq!(fields["primary_commander"]["id"], json!(545));
        assert_eq!(fields["secondary_commander"]["id"], json!(185));
        assert_eq!(fields["buffs"][0], json!({ "id": 20066, "value": 0.01 }));
    }
}
