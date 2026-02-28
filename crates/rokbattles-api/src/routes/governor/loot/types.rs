use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LootResponse {
    range: LootRange,
    total_reports: i64,
    categories: LootCategories,
}

impl LootResponse {
    pub fn new(range_start: String, range_end: String, categories: LootCategories) -> Self {
        let total_reports = categories.total_reports();

        Self {
            range: LootRange {
                start: range_start,
                end: range_end,
            },
            total_reports,
            categories,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct LootRange {
    start: String,
    end: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LootCategories {
    pub barbarian: LootCategoryAggregateResponse,
    pub barbarian_fort: LootCategoryAggregateResponse,
    pub baulur: LootCategoryAggregateResponse,
}

impl LootCategories {
    pub fn total_reports(&self) -> i64 {
        self.barbarian.reports + self.barbarian_fort.reports + self.baulur.reports
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LootCategoryAggregateResponse {
    pub reports: i64,
    pub loot_total: i64,
    pub rewards: Vec<LootRewardAggregateResponse>,
    pub daily: Vec<LootDailyAggregateResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LootRewardAggregateResponse {
    #[serde(rename = "type")]
    pub reward_type: i64,
    pub sub_type: i64,
    pub total: i64,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LootDailyAggregateResponse {
    pub date: String,
    pub reports: i64,
    pub loot_total: i64,
}
