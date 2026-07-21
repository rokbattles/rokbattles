//! DuelBattle2 output types.

use serde::{Deserialize, Serialize};
use serde_json::Number;
use ts_rs::TS;

use crate::MailMetadata;

/// Processed DuelBattle2 mail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct DuelBattle2 {
    pub metadata: MailMetadata,
    pub sender: DuelPlayer,
    pub opponent: DuelPlayer,
    pub battle_results: DuelBattleResults,
}

/// One player in an Olympian Arena duel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct DuelPlayer {
    pub player_id: u64,
    pub player_name: String,
    pub avatar_url: Option<String>,
    pub frame_url: Option<String>,
    pub alliance: DuelAlliance,
    pub duel: DuelTeam,
    pub primary_commander: DuelCommander,
    pub secondary_commander: DuelCommander,
    pub buffs: Vec<DuelBuff>,
}

/// The player's alliance identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct DuelAlliance {
    pub abbreviation: String,
}

/// The player's duel team identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct DuelTeam {
    pub team_id: u64,
}

/// A commander used in the duel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct DuelCommander {
    pub id: u64,
    pub level: u64,
    pub star_level: u64,
    pub awakened: bool,
    pub skills: Vec<DuelCommanderSkill>,
}

/// One commander skill and level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct DuelCommanderSkill {
    pub id: u64,
    pub level: u64,
}

/// One duel buff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct DuelBuff {
    pub id: u64,
    #[ts(type = "number")]
    pub value: Number,
}

/// Battle results for both duel players.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct DuelBattleResults {
    pub sender: DuelPlayerBattleResult,
    pub opponent: DuelPlayerBattleResult,
}

/// One player's duel result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct DuelPlayerBattleResult {
    pub win: bool,
    pub kill_points: u64,
    pub power: u64,
    pub units: u64,
    pub slightly_wounded: u64,
    pub severely_wounded: u64,
    pub dead: u64,
    pub heal: u64,
}
