use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingsResponse {
    pub range: PairingsRange,
    pub items: Vec<PairingAggregateResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingLoadoutsResponse {
    pub range: PairingsRange,
    pub items: Vec<PairingLoadoutAggregateResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingOpponentsResponse {
    pub range: PairingsRange,
    pub items: Vec<PairingOpponentAggregateResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingsRange {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingAggregateResponse {
    pub primary_commander_id: i64,
    pub secondary_commander_id: i64,
    pub count: i64,
    pub totals: PairingTotals,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingLoadoutAggregateResponse {
    pub key: String,
    pub count: i64,
    pub totals: PairingTotals,
    pub loadout: LoadoutSnapshot,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingOpponentAggregateResponse {
    pub enemy_primary_commander_id: i64,
    pub enemy_secondary_commander_id: i64,
    pub count: i64,
    pub totals: PairingTotals,
}

#[derive(Debug, Clone, Copy, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingTotals {
    pub kill_score: i64,
    pub deaths: i64,
    pub severely_wounded: i64,
    pub wounded: i64,
    pub enemy_kill_score: i64,
    pub enemy_deaths: i64,
    pub enemy_severely_wounded: i64,
    pub enemy_wounded: i64,
    pub dps: i64,
    pub sps: i64,
    pub tps: i64,
    pub battle_duration: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoadoutSnapshot {
    pub equipment: Vec<EquipmentToken>,
    pub armaments: Vec<LoadoutArmament>,
    pub inscriptions: Vec<i64>,
    pub formation: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EquipmentToken {
    pub slot: i64,
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub craft: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attr: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoadoutArmament {
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
}
