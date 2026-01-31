//! NPC extractor for BarCanyonKillBoss mail.

use mail_processor_sdk::{ExtractError, Extractor, Section};
use serde_json::{Map, Value};

use crate::content::{
    require_child_object, require_content, require_number_field, require_u64_field,
};

/// Extracts NPC details from BarCanyonKillBoss mail content.
#[derive(Debug, Default)]
pub struct NpcExtractor;

impl NpcExtractor {
    /// Create a new NPC extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for NpcExtractor {
    fn section(&self) -> &'static str {
        "npc"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let content = require_content(input)?;
        // mappings
        // - 102000063: Miser Khaolak
        // - 102000055: Ironhand Baulur
        let npc_type = require_u64_field(content, "npcType")?;
        let npc_level = require_u64_field(content, "npcLevel")?;
        let pos = require_child_object(content, "pos")?;
        let pos_x = require_number_field(pos, "x")?;
        let pos_y = require_number_field(pos, "y")?;

        let location = build_location(pos_x, pos_y);

        let mut section = Section::new();
        section.insert("type", Value::from(npc_type));
        section.insert("level", Value::from(npc_level));
        section.insert("location", location);
        Ok(section)
    }
}

fn build_location(x: Value, y: Value) -> Value {
    let mut location = Map::new();
    location.insert("x".to_string(), x);
    location.insert("y".to_string(), y);
    Value::Object(location)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mail_processor_sdk::Extractor;
    use serde_json::{Value, json};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn npc_extractor_reads_fields() {
        let input = json!({
            "body": {
                "content": {
                    "npcType": 102000063,
                    "npcLevel": 32,
                    "pos": { "x": 1.5, "y": 2.75 }
                }
            }
        });
        let extractor = NpcExtractor::new();
        let section = extractor.extract(&input).unwrap();

        let fields = section.fields();
        assert_eq!(fields["type"], json!(102000063));
        assert_eq!(fields["level"], json!(32));
        assert_eq!(fields["location"], json!({ "x": 1.5, "y": 2.75 }));
    }

    #[test]
    fn npc_extractor_rejects_missing_field() {
        let input = json!({
            "body": {
                "content": {
                    "npcLevel": 32,
                    "pos": { "x": 1.5, "y": 2.75 }
                }
            }
        });
        let extractor = NpcExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { .. }));
    }

    #[test]
    fn roundtrip_npc_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/BarCanyonKillBoss/Persistent.Mail.21162669176948646831.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = NpcExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(fields["type"], json!(102000055));
        assert_eq!(fields["level"], json!(25));
        assert_eq!(
            fields["location"],
            json!({ "x": 4788.31689453125, "y": 4418.36669921875 })
        );
    }
}
