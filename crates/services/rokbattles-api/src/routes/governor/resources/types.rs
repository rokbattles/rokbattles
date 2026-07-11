use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourcesResponse {
    pub range: ResourcesRange,
    pub total_reports: i64,
    pub crystals_gain: ResourceTotalsResponse,
    pub resources: Vec<ResourceTotalsByTypeResponse>,
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
pub(crate) struct ResourceTotalsByTypeResponse {
    #[serde(rename = "type")]
    pub type_id: i64,
    pub gain: i64,
    pub bonus: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceDailyValueByTypeResponse {
    #[serde(rename = "type")]
    pub type_id: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceDailyResponse {
    pub date: String,
    pub crystals_gain: i64,
    pub resources: Vec<ResourceDailyValueByTypeResponse>,
}
