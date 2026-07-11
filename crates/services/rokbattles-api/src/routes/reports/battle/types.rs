use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportsResponse {
    pub items: Vec<ReportListItem>,
    pub next_after: Option<String>,
    pub previous_before: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportByIdResponse {
    pub id: String,
    pub mail: Option<BattleReportDetail>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportListItem {
    pub mail_id: String,
    pub time_start: i64,
    pub time_end: i64,
    pub sender: ReportListParticipant,
    pub opponent: ReportListParticipant,
    pub battles: i64,
    pub kill_count: i64,
    pub trade_percent: i64,
    pub summary: ReportSummary,
    pub timeline: ReportTimeline,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportListParticipant {
    pub primary_commander_id: i64,
    pub primary_commander_awakened: Option<bool>,
    pub secondary_commander_id: i64,
    pub secondary_commander_awakened: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportSummary {
    pub sender: ReportSummaryEntry,
    pub opponent: ReportSummaryEntry,
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportSummaryEntry {
    pub troop_units: i64,
    pub dead: i64,
    pub severely_wounded: i64,
    pub slightly_wounded: i64,
    pub remaining: i64,
    pub kill_points: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportTimeline {
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub sampling: Vec<TimelineSample>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TimelineSample {
    pub tick: i64,
    pub count: i64,
}

#[derive(Debug)]
pub(crate) struct ReportRowWithCursor {
    pub mail_time: i64,
    pub item: ReportListItem,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportDetail {
    pub metadata: BattleReportMetadata,
    pub sender: BattleReportPlayer,
    pub summary: BattleReportSummary,
    pub opponents: Vec<BattleReportOpponent>,
    pub timeline: BattleReportTimeline,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportMetadata {
    pub mail_id: String,
    pub mail_time: i64,
    pub mail_role: Option<String>,
    pub kvk: Option<bool>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportPlayer {
    pub player_id: i64,
    pub player_name: String,
    pub alliance: BattleReportAlliance,
    pub avatar_url: Option<String>,
    pub frame_url: Option<String>,
    pub tracking_key: Option<String>,
    pub rally: Option<bool>,
    pub alliance_building_id: Option<i64>,
    pub castle: BattleReportCastle,
    pub app_uid: Option<i64>,
    pub commanders: BattleReportCommanderSet,
    pub support_skills: BattleReportSupportSkills,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportAlliance {
    pub abbreviation: String,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportCastle {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportCommanderSet {
    pub primary: BattleReportCommander,
    pub secondary: BattleReportCommander,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportCommander {
    pub id: Option<i64>,
    pub awakened: Option<bool>,
    pub level: Option<i64>,
    pub formation: Option<i64>,
    pub equipment: Option<String>,
    pub relics: Vec<BattleReportRelic>,
    pub skills: Vec<BattleReportCommanderSkill>,
    pub armaments: Vec<BattleReportArmament>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportRelic {
    pub id: i64,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportCommanderSkill {
    pub id: i64,
    pub level: i64,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportArmament {
    pub affix: Option<String>,
    pub buffs: Option<String>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportSupportSkills {
    pub enable: Option<bool>,
    pub skills: Vec<BattleReportSupportSkill>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportSupportSkill {
    pub hero_id: i64,
    pub skill_id: i64,
    pub skill_level: i64,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportSummary {
    pub sender: BattleReportSummaryEntry,
    pub opponent: BattleReportSummaryEntry,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportSummaryEntry {
    pub kill_points: Option<i64>,
    pub dead: Option<i64>,
    pub severely_wounded: Option<i64>,
    pub slightly_wounded: Option<i64>,
    pub remaining: Option<i64>,
    pub troop_units: Option<i64>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportOpponent {
    pub player_id: i64,
    pub player_name: String,
    pub alliance: BattleReportAlliance,
    pub avatar_url: Option<String>,
    pub frame_url: Option<String>,
    pub tracking_key: Option<String>,
    pub rally: Option<bool>,
    pub alliance_building_id: Option<i64>,
    pub castle: BattleReportCastle,
    pub app_uid: Option<i64>,
    pub commanders: BattleReportCommanderSet,
    pub start_tick: i64,
    pub end_tick: i64,
    pub attack: BattleReportAttack,
    pub npc: BattleReportNpc,
    pub battle_results: BattleReportBattleResults,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportAttack {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportNpc {
    pub r#type: Option<i64>,
    pub b_type: Option<i64>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportBattleResults {
    pub sender: BattleReportBattleResult,
    pub opponent: BattleReportBattleResult,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportBattleResult {
    pub reinforcements_join: Option<i64>,
    pub reinforcements_leave: Option<i64>,
    pub kill_points: Option<i64>,
    pub acclaim: Option<i64>,
    pub severely_wounded: Option<i64>,
    pub slightly_wounded: Option<i64>,
    pub remaining: Option<i64>,
    pub dead: Option<i64>,
    pub heal: Option<i64>,
    pub troop_units: Option<i64>,
    pub troop_units_max: Option<i64>,
    pub power: Option<i64>,
    pub attack_power: Option<i64>,
    pub skill_power: Option<i64>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BattleReportTimeline {
    pub start_timestamp: i64,
    pub start_tick: i64,
}
