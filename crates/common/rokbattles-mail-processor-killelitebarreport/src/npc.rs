//! Copies `npcType`, `npcLevel`, and `pos` into an `npc` object.
//!
//! Type and level must be unsigned integers; coordinates may be any JSON numbers.
//! NPC IDs are preserved as supplied rather than mapped to a fixed set of bosses.

use rokbattles_mail_sdk::{ExtractError, Extractor, Section};
use serde_json::{Map, Value};

use crate::content::{
    require_child_object, require_content, require_number_field, require_u64_field,
};

/// Extracts NPC details out of KillEliteBarReport mail content.
#[derive(Debug, Default)]
pub struct NpcExtractor;

impl NpcExtractor {
    /// Creates an NPC extractor.
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
