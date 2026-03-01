use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourcesResponse {
    pub range: ResourcesRange,
    pub total_reports: i64,
    pub breakdown: ResourceBreakdownResponse,
    pub daily: Vec<ResourceDailyResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourcesRange {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Copy, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceTotalsResponse {
    pub gain: i64,
    pub bonus: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceBreakdownResponse {
    pub crystals: ResourceTotalsResponse,
    pub food: ResourceTotalsResponse,
    pub wood: ResourceTotalsResponse,
    pub stone: ResourceTotalsResponse,
    pub gold: ResourceTotalsResponse,
    pub gems: ResourceTotalsResponse,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceDailyResponse {
    pub date: String,
    pub crystals: i64,
    pub food: i64,
    pub wood: i64,
    pub stone: i64,
    pub gold: i64,
    pub gems: i64,
}
