use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuelBattle2Response {
    pub items: Vec<DuelBattle2ListItem>,
    pub next_after: Option<String>,
    pub previous_before: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuelBattle2ListItem {
    pub duel_id: i64,
    pub win_streak: i64,
    pub mail_time: i64,
    pub kill_count: i64,
    pub trade_percent: i64,
    pub entry: DuelBattle2Entry,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuelBattle2Entry {
    pub sender: DuelBattle2Participant,
    pub opponent: DuelBattle2Participant,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuelBattle2Participant {
    pub primary_commander_id: i64,
    pub primary_commander_awakened: Option<bool>,
    pub secondary_commander_id: i64,
    pub secondary_commander_awakened: Option<bool>,
}

#[derive(Debug)]
pub(crate) struct DuelBattle2RowWithCursor {
    pub latest_mail_time: i64,
    pub item: DuelBattle2ListItem,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuelBattle2DetailResponse {
    pub items: Vec<DuelBattle2DetailItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuelBattle2DetailItem {
    pub metadata: DuelBattle2DetailMetadata,
    pub sender: DuelBattle2DetailPlayer,
    pub opponent: DuelBattle2DetailPlayer,
    pub battle_results: DuelBattle2DetailBattleResults,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuelBattle2DetailMetadata {
    pub mail_id: String,
    pub mail_time: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuelBattle2DetailPlayer {
    pub player_id: i64,
    pub player_name: String,
    pub avatar_url: Option<String>,
    pub frame_url: Option<String>,
    pub alliance: DuelBattle2DetailAlliance,
    pub primary_commander: DuelBattle2DetailCommander,
    pub secondary_commander: DuelBattle2DetailCommander,
    pub buffs: Vec<DuelBattle2DetailBuff>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuelBattle2DetailAlliance {
    pub abbreviation: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuelBattle2DetailCommander {
    pub id: i64,
    pub awakened: Option<bool>,
    pub level: i64,
    pub skills: Vec<DuelBattle2DetailCommanderSkill>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuelBattle2DetailCommanderSkill {
    pub id: i64,
    pub level: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuelBattle2DetailBuff {
    pub id: i64,
    pub value: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuelBattle2DetailBattleResults {
    pub sender: DuelBattle2DetailBattleResult,
    pub opponent: DuelBattle2DetailBattleResult,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuelBattle2DetailBattleResult {
    pub win: bool,
    pub kill_points: i64,
    pub power: i64,
    pub units: i64,
    pub slightly_wounded: i64,
    pub severely_wounded: i64,
    pub dead: i64,
    pub heal: i64,
}
