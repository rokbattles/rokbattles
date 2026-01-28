//! Sender extractor for DuelBattle2 mail.

use mail_processor_sdk::{ExtractError, Extractor, Section};
use serde_json::Value;

use crate::commander::extract_player_commanders;
use crate::player::{extract_player_buffs, extract_player_section_from_map, locate_player};

/// Extracts sender details from the attacking player data.
#[derive(Debug, Default)]
pub struct SenderExtractor;

impl SenderExtractor {
    /// Create a new sender extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for SenderExtractor {
    fn section(&self) -> &'static str {
        "sender"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let player = locate_player(input, "AtkPlayer")?;
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
    fn sender_extractor_handles_string_avatar() {
        let input = json!({
            "body": {
                "detail": {
                    "AtkPlayer": {
                        "PlayerId": 100,
                        "PlayerName": "Attacker",
                        "PlayerAvatar": "https://example.com/avatar.png",
                        "Abbr": "ATK",
                        "DuelTeamId": 501,
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
        let extractor = SenderExtractor::new();
        let section = extractor.extract(&input).unwrap();

        let fields = section.fields();
        assert_eq!(fields["player_id"], json!(100));
        assert_eq!(fields["player_name"], json!("Attacker"));
        assert_eq!(
            fields["avatar_url"],
            json!("https://example.com/avatar.png")
        );
        assert_eq!(fields["frame_url"], json!(null));
        assert_eq!(fields["alliance"], json!({ "abbreviation": "ATK" }));
        assert_eq!(fields["duel"], json!({ "team_id": 501 }));
        assert_eq!(fields["primary_commander"]["id"], json!(10));
        assert_eq!(fields["secondary_commander"]["id"], json!(20));
        assert_eq!(fields["buffs"][0], json!({ "id": 10, "value": 1.25 }));
    }

    #[test]
    fn roundtrip_sender_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/DuelBattle2/Persistent.Mail.4197312176618249531.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = SenderExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(fields["player_id"], json!(71738515));
        assert_eq!(fields["player_name"], json!("G Var"));
        assert_eq!(
            fields["avatar_url"],
            json!(
                "https://plat-fau-global.lilithgame.com/p/astc/IM/10043/0/32624247/2025-12-07/A148C6B7BAF717C52970B3B4476AB77D_250x250.jpg"
            )
        );
        assert_eq!(
            fields["frame_url"],
            json!("http://imimg.lilithcdn.com/roc/img_AvatarFrame_16.png")
        );
        assert_eq!(fields["alliance"], json!({ "abbreviation": "SO4L" }));
        assert_eq!(fields["duel"], json!({ "team_id": 1357157902 }));
        assert_eq!(fields["primary_commander"]["id"], json!(540));
        assert_eq!(fields["secondary_commander"]["id"], json!(575));
        assert_eq!(fields["buffs"][0], json!({ "id": 5512, "value": 6 }));
    }
}
