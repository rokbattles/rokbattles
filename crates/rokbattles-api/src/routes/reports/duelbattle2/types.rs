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
    pub secondary_commander_id: i64,
}

#[derive(Debug)]
pub(crate) struct DuelBattle2RowWithCursor {
    pub latest_mail_time: i64,
    pub item: DuelBattle2ListItem,
}
