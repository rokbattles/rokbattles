//! AllianceAOOBattleResults output types.

use serde::{Deserialize, Serialize};
use serde_json::Number;
use ts_rs::TS;

use crate::MailMetadata;

/// Processed AllianceAOOBattleResults mail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AllianceAooBattleResults {
    pub metadata: MailMetadata,
    pub alliances: Vec<BattleAllianceResult>,
    pub body: AllianceBattleBody,
    pub participants: Vec<AllianceBattleParticipant>,
    pub overview: AllianceBattleOverview,
}

/// One alliance's match result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleAllianceResult {
    pub alliance: BattleAlliance,
    pub members: u64,
    pub members_max: u64,
    pub power: Option<u64>,
    pub score: u64,
    pub server_id: Option<u64>,
    pub is_blue: bool,
    pub team: Option<u64>,
}

/// Alliance identity included in battle results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleAlliance {
    pub abbreviation: Option<String>,
    pub id: u64,
    pub name: Option<String>,
    pub logo: Option<String>,
}

/// Match-level flags and alliance identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AllianceBattleBody {
    pub r#type: u64,
    pub param: Option<u64>,
    pub battle_type: String,
    pub win: bool,
    pub alliance: AllianceId,
}

/// Alliance identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AllianceId {
    pub id: u64,
}

/// One participant's score line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AllianceBattleParticipant {
    pub player_name: String,
    #[ts(type = "number")]
    pub individual_points: Number,
    #[ts(type = "number")]
    pub building_score: Number,
    #[ts(type = "number")]
    pub gather_score: Number,
    #[ts(type = "number")]
    pub kill_score: Number,
    #[ts(type = "number")]
    pub flag_score: Number,
}

/// Category leaders and alliance totals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AllianceBattleOverview {
    pub flag_score: ScoreCategory,
    pub building_score: ScoreCategory,
    pub be_killed_score: ScoreCategory,
    pub gather_score: ScoreCategory,
    pub healing_score: ScoreCategory,
    pub killed_score: ScoreCategory,
}

/// One score category's alliance total and optional MVP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ScoreCategory {
    #[ts(type = "number")]
    pub alliance_score: Number,
    pub mvp: Option<CategoryMvp>,
}

/// The highest-scoring player for one category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CategoryMvp {
    pub player_id: u64,
    pub player_name: String,
    #[ts(type = "number")]
    pub score: Number,
}
