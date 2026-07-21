//! AllianceAOOBattleInfo output types.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{MailMetadata, Reward};

/// Processed AllianceAOOBattleInfo mail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AllianceAooBattleInfo {
    pub metadata: MailMetadata,
    pub rewards: Vec<Reward>,
    pub body: BattleInfoBody,
}

/// Scheduled Ark of Osiris fights.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleInfoBody {
    pub fights: Vec<ScheduledFight>,
}

/// One scheduled fight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ScheduledFight {
    pub team: u64,
    pub time: u64,
    pub win: bool,
}
