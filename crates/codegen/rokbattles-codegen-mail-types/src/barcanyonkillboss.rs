//! BarCanyonKillBoss output types.

use serde::{Deserialize, Serialize};
use serde_json::Number;
use ts_rs::TS;

use crate::{MailMetadata, Position, Reward};

/// Processed BarCanyonKillBoss mail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BarCanyonKillBoss {
    pub metadata: MailMetadata,
    pub npc: CanyonNpc,
    pub participants: Vec<CanyonParticipant>,
}

/// Defeated canyon NPC details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CanyonNpc {
    pub r#type: u64,
    pub level: u64,
    pub location: Position,
}

/// One canyon participant and their loot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CanyonParticipant {
    pub player_id: u64,
    pub player_name: String,
    pub avatar_url: Option<String>,
    pub frame_url: Option<String>,
    #[ts(type = "number")]
    pub damage_rate: Number,
    pub loot: Vec<Reward>,
}
