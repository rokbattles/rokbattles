//! EventMemberLootReport output types.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{MailMetadata, Reward};

/// Processed EventMemberLootReport mail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct EventMemberLootReport {
    pub metadata: MailMetadata,
    pub boss: AllianceBoss,
    pub participants: Vec<AllianceBossParticipant>,
}

/// Alliance boss identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AllianceBoss {
    pub id: u64,
}

/// One alliance boss participant and their loot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AllianceBossParticipant {
    pub player_id: u64,
    pub player_name: String,
    pub avatar_url: Option<String>,
    pub frame_url: Option<String>,
    pub loot: Vec<Reward>,
}
