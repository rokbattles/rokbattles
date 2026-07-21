//! AllianceAOOIndividualResults output types.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{MailMetadata, Reward};

/// Processed AllianceAOOIndividualResults mail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AllianceAooIndividualResults {
    pub metadata: MailMetadata,
    pub rewards: Vec<Reward>,
    pub body: IndividualBattleBody,
    pub overview: IndividualBattleOverview,
    pub pairings: Vec<CommanderPairing>,
    pub results: IndividualBattleResults,
}

/// Match-level flags for the individual result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct IndividualBattleBody {
    pub r#type: u64,
    pub param: Option<u64>,
    pub win: bool,
    pub team: Option<u64>,
}

/// Rank and aggregate player results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct IndividualBattleOverview {
    pub player_name: Option<String>,
    pub player_id: Option<u64>,
    pub score: Option<u64>,
    pub rank: u64,
    pub total_results: Option<TotalBattleResults>,
}

/// Aggregate battle totals included with the rank.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct TotalBattleResults {
    pub battles: u64,
    pub kill_points: u64,
    pub severely_wounded: u64,
}

/// Results for one commander pairing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CommanderPairing {
    pub primary_commander: CommanderId,
    pub secondary_commander: CommanderId,
    pub kill_count: u64,
    pub battles_win: u64,
    pub battles: u64,
    pub severely_wounded: u64,
    pub kill_points: u64,
}

/// Commander identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CommanderId {
    pub id: u64,
}

/// High-level individual match results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct IndividualBattleResults {
    pub total_score: Option<u64>,
    pub win_rate: Option<u64>,
    pub battles_win: Option<u64>,
    pub battles_lose: Option<u64>,
    pub severely_wounded: Option<u64>,
    pub kills: Option<u64>,
    pub kill_score: Option<u64>,
    pub flag_score: Option<u64>,
    pub building_score: Option<u64>,
    pub gather_score: Option<u64>,
    pub healing_score: Option<u64>,
    pub units_healed: Option<u64>,
    pub flag_count: Option<u64>,
    pub teleports: Option<u64>,
    pub speedups: Option<u64>,
    pub structures: Option<u64>,
}
