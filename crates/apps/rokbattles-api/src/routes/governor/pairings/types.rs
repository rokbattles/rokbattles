use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingsResponse {
    pub range: PairingsRange,
    pub items: Vec<PairingAggregateResponse>,
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
    pub positive_trades: i64,
    pub totals: PairingTotals,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub(crate) struct PairingTotals {
    pub kill_score: i64,
    pub deaths: i64,
    pub severely_wounded: i64,
    pub wounded: i64,
    pub power_loss: i64,
    pub atk_power_loss: i64,
    pub skill_power_loss: i64,
    pub enemy_kill_score: i64,
    pub enemy_deaths: i64,
    pub enemy_severely_wounded: i64,
    pub enemy_wounded: i64,
    pub enemy_power_loss: i64,
    pub enemy_atk_power_loss: i64,
    pub enemy_skill_power_loss: i64,
    pub dps: i64,
    pub sps: i64,
    pub tps: i64,
    pub hps: f64,
    pub weighted_trade_percent: f64,
    pub battle_duration: i64,
    pub(crate) healing_count: i64,
    pub enemy_healing_count: i64,
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
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingStats {
    pub battles: i64,
    pub kp_gain: i64,
    pub kp_loss: i64,
    pub net_kp: i64,
    pub trade_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingDetailLoadout {
    pub key: String,
    pub stats: PairingStats,
    pub loadout: LoadoutSnapshot,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingDetailResponse {
    pub range: PairingsRange,
    pub pairing: Option<PairingStats>,
    pub loadouts: Vec<PairingDetailLoadout>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingMatchup {
    pub primary_commander_id: i64,
    pub secondary_commander_id: i64,
    pub stats: PairingStats,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingMatchupsResponse {
    pub range: PairingsRange,
    pub items: Vec<PairingMatchup>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingDurationDistribution {
    pub count: usize,
    pub p10_millis: f64,
    pub p25_millis: f64,
    pub p50_millis: f64,
    pub p75_millis: f64,
    pub p90_millis: f64,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingPerformanceMetrics {
    pub battles: i64,
    pub battle_duration_millis: i64,
    pub duration_distribution: Option<PairingDurationDistribution>,
    pub kp_gain: i64,
    pub kp_loss: i64,
    pub sevs_gain: i64,
    pub sevs_loss: i64,
    pub power_loss: i64,
    pub opponent_power_loss: i64,
    pub trade_percent: Option<f64>,
    pub dps: Option<f64>,
    pub sps: Option<f64>,
    pub tps: Option<f64>,
    pub hps: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingPerformancePoint {
    pub date: String,
    pub metrics: PairingPerformanceMetrics,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingPerformanceResponse {
    pub range: PairingsRange,
    pub totals: PairingPerformanceMetrics,
    pub series: Vec<PairingPerformancePoint>,
}
