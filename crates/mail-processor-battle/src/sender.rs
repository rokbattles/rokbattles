//! Sender extractor for Battle mail.

use mail_processor_sdk::{ExtractError, Extractor, Section};
use serde_json::Value;

use crate::content::{require_child_object, require_content};
use crate::participants::extract_participants;
use crate::player::extract_player_fields;

/// Extracts sender details from the SelfChar payload.
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
        let content = require_content(input)?;
        let sender = require_child_object(content, "SelfChar")?;
        let fields = extract_player_fields(sender)?;
        let participants = extract_participants(content, "STs")?;

        let mut section = Section::new();
        for (key, value) in fields {
            section.insert(key, value);
        }
        section.insert("participants", participants);
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

    fn load_sample() -> Value {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Battle/Persistent.Mail.1002579517552941234.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        serde_json::from_str(&json).expect("parse sample")
    }

    #[test]
    fn sender_extractor_reads_basic_fields() {
        let input = json!({
            "body": {
                "content": {
                    "STs": {
                        "1": {
                            "PId": 10,
                            "PName": "Sender",
                            "Abbr": "AL",
                            "HId": 11,
                            "HLv": 20,
                            "HId2": 12,
                            "HLv2": 21
                        }
                    },
                    "SelfChar": {
                        "PId": 10,
                        "PName": "Sender",
                        "COSId": 5,
                        "AId": 42,
                        "AName": "Alliance",
                        "Abbr": "AL",
                        "AbT": 1,
                        "CastlePos": { "X": 100.5, "Y": 200.25 },
                        "CastleLevel": 12,
                        "CTK": "tracking-3",
                        "ShId": 25,
                        "IsRally": true,
                        "AppUid": "8518744-123",
                        "Avatar": "{\"avatar\":\"https://example.com/avatar.png\",\"avatarFrame\":null}"
                    }
                }
            }
        });
        let extractor = SenderExtractor::new();
        let section = extractor.extract(&input).unwrap();

        let fields = section.fields();
        assert_eq!(fields["player_id"], json!(10));
        assert_eq!(fields["player_name"], json!("Sender"));
        assert_eq!(fields["kingdom_id"], json!(5));
        assert_eq!(
            fields["alliance"],
            json!({ "id": 42, "name": "Alliance", "abbreviation": "AL" })
        );
        assert_eq!(fields["alliance_building_id"], json!(1));
        assert_eq!(
            fields["castle"],
            json!({
                "x": 100.5,
                "y": 200.25,
                "level": 12,
                "watchtower": null
            })
        );
        assert_eq!(fields["tracking_key"], json!("tracking-3"));
        assert_eq!(fields["camp_id"], json!(null));
        assert_eq!(fields["rally"], json!(true));
        assert_eq!(fields["structure_id"], json!(25));
        assert!(fields["commanders"]["primary"]["id"].is_null());
        assert!(fields["commanders"]["secondary"]["id"].is_null());
        assert!(fields["commanders"]["primary"]["armaments"].is_null());
        assert_eq!(fields["app_id"], json!(8518744));
        assert_eq!(fields["app_uid"], json!(123));
        assert_eq!(
            fields["avatar_url"],
            json!("https://example.com/avatar.png")
        );
        assert_eq!(fields["frame_url"], json!(null));
        let participants = fields["participants"]
            .as_array()
            .expect("participants array");
        assert_eq!(participants.len(), 1);
        assert_eq!(
            participants[0],
            json!({
                "participant_id": 1,
                "player_id": 10,
                "player_name": "Sender",
                "alliance": { "abbreviation": "AL" },
                "commanders": {
                    "primary": { "id": 11, "level": 20 },
                    "secondary": { "id": 12, "level": 21 },
                }
            })
        );
    }

    #[test]
    fn sender_extractor_rejects_missing_self_char() {
        let input = json!({ "body": { "content": {} } });
        let extractor = SenderExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { .. }));
    }

    #[test]
    fn sender_extractor_defaults_missing_tracking_key() {
        let input = json!({
            "body": {
                "content": {
                    "SelfChar": {
                        "PId": 10,
                        "PName": "Sender",
                        "COSId": 5,
                        "AId": 42,
                        "AName": "Alliance",
                        "Abbr": "AL",
                        "CastlePos": { "X": 100.5, "Y": 200.25 },
                        "CastleLevel": 12,
                        "Avatar": "null"
                    }
                }
            }
        });
        let extractor = SenderExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();
        assert_eq!(fields["tracking_key"], json!(""));
    }

    #[test]
    fn roundtrip_sender_extracts_core_fields() {
        let value = load_sample();
        let extractor = SenderExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(fields["player_id"], json!(110176153));
        assert_eq!(fields["player_name"], json!("F7yst"));
        assert_eq!(fields["kingdom_id"], json!(1804));
        assert_eq!(fields["alliance"]["id"], json!(4805441));
        assert_eq!(fields["alliance"]["abbreviation"], json!("SO4L"));
        assert!(fields["alliance"]["name"].is_string());
        assert!(fields["alliance_building_id"].is_null());
        assert_eq!(fields["tracking_key"], json!("110176153_1755294119_116_15"));
        assert_eq!(
            fields["castle"],
            json!({
                "x": 2200.8281,
                "y": 6305.255,
                "level": 25,
                "watchtower": null
            })
        );
        assert_eq!(fields["camp_id"], json!(null));
        assert!(fields["rally"].is_null());
        assert!(fields["structure_id"].is_null());
        let participants = fields["participants"]
            .as_array()
            .expect("participants array");
        assert_eq!(participants.len(), 1);
        assert_eq!(participants[0]["participant_id"], json!(575753));
        assert_eq!(participants[0]["player_id"], json!(110176153));
        assert_eq!(participants[0]["player_name"], json!("F7yst"));
        assert_eq!(
            participants[0]["alliance"],
            json!({ "abbreviation": "SO4L" })
        );
    }

    #[test]
    fn roundtrip_sender_extracts_commanders() {
        let value = load_sample();
        let extractor = SenderExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(fields["commanders"]["primary"]["id"], json!(116));
        assert_eq!(fields["commanders"]["primary"]["level"], json!(50));
        assert_eq!(fields["commanders"]["primary"]["awakened"], json!(true));
        assert_eq!(fields["commanders"]["primary"]["star_level"], json!(5));
        assert_eq!(fields["commanders"]["primary"]["relics"], json!(null));
        assert_eq!(fields["commanders"]["secondary"]["relics"], json!(null));
        assert_eq!(fields["commanders"]["secondary"]["armaments"], json!(null));
        let armaments = fields["commanders"]["primary"]["armaments"]
            .as_array()
            .expect("armaments array");
        assert_eq!(armaments.len(), 4);
        assert_eq!(armaments[0]["id"], json!(1));
        assert_eq!(fields["commanders"]["secondary"]["id"], json!(15));
        assert_eq!(fields["commanders"]["secondary"]["awakened"], json!(false));
        assert_eq!(fields["commanders"]["secondary"]["star_level"], json!(4));
        assert_eq!(fields["app_id"], json!(2104267));
        assert_eq!(fields["app_uid"], json!(88504567));
        assert_eq!(
            fields["avatar_url"],
            json!(
                "https://imimg.lilithcdn.com/roc/llc_avatar/110176153/23/08/22/4d3650bd6339b4c4_640x640.jpg"
            )
        );
        assert_eq!(
            fields["frame_url"],
            json!("http://imimg.lilithcdn.com/roc/img_ProfileBg220x220_a.png")
        );
    }
}
