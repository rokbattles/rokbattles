use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
struct LootRange {
    start: String,
    end: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PersonalLootResponse {
    range: LootRange,
    totals: PersonalLootTotals,
    groups: Vec<PersonalLootGroupResponse>,
}

impl PersonalLootResponse {
    pub fn new(
        range_start: String,
        range_end: String,
        groups: Vec<PersonalLootGroupResponse>,
    ) -> Self {
        let totals = groups.iter().fold(PersonalLootTotals::default(), |mut totals, group| {
            totals.results += group.reports;
            totals.ap_used += group.ap_used;
            totals.honor_gained += group.honor_gained;
            totals.xp_gained += group.xp_gained;
            totals
        });

        Self { range: LootRange { start: range_start, end: range_end }, totals, groups }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PersonalLootTotals {
    pub results: i64,
    pub ap_used: i64,
    pub honor_gained: i64,
    pub xp_gained: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PersonalLootGroupResponse {
    pub level: Option<i32>,
    pub reports: i64,
    pub loot_total: i64,
    pub ap_used: i64,
    pub honor_gained: i64,
    pub xp_gained: i64,
    pub rewards: Vec<LootRewardAggregateResponse>,
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
