use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArkHistoryResponse {
    pub limit: i64,
    pub total: i64,
    pub items: Vec<ArkMatchSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArkDetailResponse {
    pub id: String,
    #[serde(rename = "match")]
    pub ark_match: Option<ArkMatchDetail>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArkMatchSummary {
    pub match_id: String,
    pub mail_time_millis: i64,
    pub battle_results_mail_id: Option<String>,
    pub battle_info_mail_id: Option<String>,
    pub individual_results_mail_id: Option<String>,
    pub alliances: Vec<ArkMatchAlliance>,
    pub winner_alliance_id: Option<i64>,
    pub has_battle_info: bool,
    pub has_individual_results: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArkMatchAlliance {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub abbreviation: Option<String>,
    pub score: Option<i64>,
    pub members: Option<i64>,
    pub members_max: Option<i64>,
    pub is_blue: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArkMatchDetail {
    #[serde(flatten)]
    pub summary: ArkMatchSummary,
    pub overview: ArkMatchDetailOverview,
    pub individual_results: ArkMatchDetailIndividualResults,
    pub pairings: Vec<ArkMatchDetailPairing>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArkMatchDetailOverview {
    pub rank: Option<i64>,
    pub score: Option<i64>,
    pub battles: Option<i64>,
    pub kill_points_gain: Option<i64>,
    pub kill_points_loss: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArkMatchDetailIndividualResults {
    pub battles_win: Option<i64>,
    pub battles_lose: Option<i64>,
    pub win_rate: Option<i64>,
    pub kills: Option<i64>,
    pub severely_wounded: Option<i64>,
    pub units_healed: Option<i64>,
    pub speedups: Option<i64>,
    pub teleports: Option<i64>,
    pub structures: Option<i64>,
    pub provisions_score: Option<i64>,
    pub ark_of_osiris_score: Option<i64>,
    pub kill_score: Option<i64>,
    pub occupation_score: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArkMatchDetailPairing {
    pub primary_commander_id: Option<i64>,
    pub secondary_commander_id: Option<i64>,
    pub battles: Option<i64>,
    pub battles_win: Option<i64>,
    pub kill_count: Option<i64>,
    pub kill_points: Option<i64>,
    pub severely_wounded: Option<i64>,
}
