//! Battle output types.

use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use ts_rs::TS;

use crate::Reward;

/// Processed Battle mail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Battle {
    pub metadata: BattleMetadata,
    pub sender: BattlePlayer,
    pub summary: BattleSummary,
    pub opponents: Vec<BattleOpponent>,
    pub timeline: BattleTimeline,
}

/// Battle-specific mail metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleMetadata {
    pub mail_id: String,
    pub mail_time: u64,
    pub mail_receiver: String,
    pub server_id: u64,
    pub report_id: u64,
    pub mail_role: String,
    pub kvk: bool,
    pub room_id: Option<u64>,
    pub schema: Option<u64>,
    pub ll_script_schema: Option<u64>,
}

/// The player who initiated the battle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattlePlayer {
    pub player_id: i64,
    pub player_name: String,
    pub kingdom_id: Option<u64>,
    pub alliance: BattleAlliance,
    pub alliance_building_id: Option<u64>,
    pub character_type: Option<u64>,
    pub is_turret: Option<bool>,
    pub is_outpost: Option<bool>,
    pub castle: BattleCastle,
    pub tracking_key: String,
    pub camp_id: Option<u64>,
    pub rally: Option<bool>,
    pub structure_id: Option<u64>,
    pub commanders: BattleCommanders,
    pub support_skills: BattleSupportSkills,
    pub auxiliary_skills: Vec<BattleAuxiliarySkill>,
    pub app_id: Option<u64>,
    pub app_uid: Option<u64>,
    pub server_season: Option<String>,
    pub package_identifier: Option<String>,
    pub as_battle_type: Option<u64>,
    pub session: Option<String>,
    pub avatar_url: Option<String>,
    pub frame_url: Option<String>,
    pub supreme_strife: SupremeStrife,
    pub participants: Vec<BattleParticipant>,
}

/// One opponent and the attack against them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleOpponent {
    pub player_id: i64,
    pub player_name: String,
    pub kingdom_id: Option<u64>,
    pub alliance: BattleAlliance,
    pub alliance_building_id: Option<u64>,
    pub character_type: Option<u64>,
    pub is_turret: Option<bool>,
    pub is_outpost: Option<bool>,
    pub castle: BattleCastle,
    pub tracking_key: String,
    pub camp_id: Option<u64>,
    pub rally: Option<bool>,
    pub structure_id: Option<u64>,
    pub commanders: BattleCommanders,
    pub support_skills: BattleSupportSkills,
    pub auxiliary_skills: Vec<BattleAuxiliarySkill>,
    pub app_id: Option<u64>,
    pub app_uid: Option<u64>,
    pub server_season: Option<String>,
    pub package_identifier: Option<String>,
    pub as_battle_type: Option<u64>,
    pub session: Option<String>,
    pub avatar_url: Option<String>,
    pub frame_url: Option<String>,
    pub supreme_strife: SupremeStrife,
    pub attack: BattleAttack,
    pub start_tick: u64,
    pub end_tick: u64,
    pub participants: Vec<BattleParticipant>,
    pub npc: BattleNpc,
    pub battle_results: BattleResults,
    pub battle_effects: BattleEffects,
}

/// Alliance identity used by a battle player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleAlliance {
    pub id: u64,
    pub name: String,
    pub abbreviation: String,
}

/// Castle position and levels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleCastle {
    #[ts(type = "number")]
    pub x: Number,
    #[ts(type = "number")]
    pub y: Number,
    pub level: u64,
    pub watchtower: Option<u64>,
}

/// Primary and secondary commanders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleCommanders {
    pub primary: BattleCommander,
    pub secondary: BattleCommander,
}

/// Commander details normalized from a battle player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleCommander {
    pub id: Option<u64>,
    pub level: Option<u64>,
    pub formation: Option<u64>,
    pub awakened: Option<bool>,
    pub star_level: Option<u64>,
    pub equipment: Option<String>,
    pub skills: Option<Vec<BattleCommanderSkill>>,
    pub relics: Option<Vec<BattleRelic>>,
    pub armaments: Option<Vec<BattleArmament>>,
}

/// One commander skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleCommanderSkill {
    pub id: u64,
    pub level: u64,
}

/// One commander relic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleRelic {
    pub id: u64,
}

/// One commander armament.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleArmament {
    pub id: u64,
    pub affix: String,
    pub buffs: String,
}

/// Support-skill configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleSupportSkills {
    pub enable: bool,
    pub skills: Vec<BattleSupportSkill>,
}

/// One support skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleSupportSkill {
    pub hero_id: u64,
    pub skill_id: u64,
    pub skill_level: u64,
}

/// One auxiliary commander skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleAuxiliarySkill {
    pub hero_id: u64,
    pub level: u64,
    pub skill_id: u64,
}

/// Supreme Strife identifiers attached to a player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct SupremeStrife {
    pub battle_id: Option<String>,
    pub team_id: Option<u64>,
    pub round: Option<u64>,
}

/// One reinforcement participant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleParticipant {
    pub participant_id: i64,
    pub player_id: i64,
    pub player_name: String,
    pub alliance: ParticipantAlliance,
    pub commanders: ParticipantCommanders,
}

/// Alliance identity included for a participant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ParticipantAlliance {
    pub abbreviation: String,
}

/// Participant commander identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ParticipantCommanders {
    pub primary: ParticipantCommander,
    pub secondary: ParticipantCommander,
}

/// A participant commander identifier and level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ParticipantCommander {
    pub id: Option<u64>,
    pub level: Option<u64>,
}

/// Attack identity and map position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleAttack {
    pub id: String,
    #[ts(type = "number")]
    pub x: Number,
    #[ts(type = "number")]
    pub y: Number,
}

/// NPC details attached to an attack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleNpc {
    pub r#type: Option<u64>,
    pub b_type: Option<u64>,
    pub experience: Option<u64>,
    pub loot: Option<Vec<Reward>>,
}

/// Battle-result values for both sides of an attack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleResults {
    pub sender: BattleResult,
    pub opponent: BattleResult,
}

/// One side's normalized battle results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleResult {
    pub reinforcements_join: Option<u64>,
    pub reinforcements_leave: Option<u64>,
    pub kill_points: Option<u64>,
    pub acclaim: Option<u64>,
    pub severely_wounded: Option<u64>,
    pub slightly_wounded: Option<i64>,
    pub remaining: Option<u64>,
    pub dead: Option<u64>,
    pub heal: Option<u64>,
    pub troop_units: Option<u64>,
    pub troop_units_max: Option<u64>,
    pub watchtower_max: Option<u64>,
    pub watchtower: Option<u64>,
    pub power: Option<i64>,
    pub attack_power: Option<i64>,
    pub skill_power: Option<i64>,
    pub merits: Option<u64>,
    pub death_reduction: Option<u64>,
    pub severe_wound_reduction: Option<u64>,
}

/// Modifier and statistic effects for both battle sides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleEffects {
    pub sender: BattleEffectSide,
    pub opponent: BattleEffectSide,
}

/// Effects attached to one battle side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleEffectSide {
    pub modifier_sources: Vec<BattleModifierSource>,
    pub statistics: Vec<BattleEffectStatistic>,
}

/// Modifier identifiers grouped by their source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleModifierSource {
    pub source: String,
    pub ids: Vec<u64>,
}

/// One effect statistic and its values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleEffectStatistic {
    pub source: String,
    pub id: u64,
    pub stats: Vec<BattleEffectValue>,
}

/// One effect-statistic value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleEffectValue {
    pub key: String,
    #[ts(type = "unknown")]
    pub value: Value,
}

/// Summary results for both battle sides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleSummary {
    pub sender: BattleSummarySide,
    pub opponent: BattleSummarySide,
}

/// One side's summary values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleSummarySide {
    pub kill_points: Option<u64>,
    pub dead: Option<u64>,
    pub severely_wounded: Option<u64>,
    pub slightly_wounded: Option<i64>,
    pub remaining: Option<u64>,
    pub troop_units: Option<u64>,
}

/// Battle samples and reinforcement events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleTimeline {
    pub start_timestamp: u64,
    pub end_timestamp: u64,
    pub start_tick: u64,
    pub sampling: Vec<BattleSample>,
    pub events: Vec<BattleTimelineEvent>,
}

/// One unit-count sample.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleSample {
    pub tick: u64,
    pub count: u64,
}

/// One reinforcement event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BattleTimelineEvent {
    pub tick: u64,
    pub r#type: u64,
    pub event_id: Option<u64>,
    pub player_id: i64,
    pub player_name: String,
    pub count: Option<u64>,
    pub avatar_url: Option<String>,
    pub frame_url: Option<String>,
    pub commanders: ParticipantCommanders,
}
