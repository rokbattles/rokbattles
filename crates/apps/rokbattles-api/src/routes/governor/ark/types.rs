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
    pub alliances: Vec<ArkMatchAlliance>,
    pub winner_alliance_id: Option<i64>,
    pub self_alliance_id: Option<i64>,
    pub league: ArkLeague,
    pub personal_score: Option<i64>,
    pub has_individual_results: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArkMatchAlliance {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub logo: Option<String>,
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
    pub participants: Vec<ArkParticipant>,
    pub highlights: Vec<ArkHighlight>,
    pub battle_reports: ArkReportCoverage,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArkMatchDetailOverview {
    pub rank: Option<i64>,
    pub score: Option<i64>,
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
    pub speedups_minutes: Option<i64>,
    pub teleports: Option<i64>,
    pub provisions_score: Option<i64>,
    pub ark_of_osiris_score: Option<i64>,
    pub kill_score: Option<i64>,
    pub occupation_score: Option<i64>,
    pub healing_score: Option<i64>,
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
    pub loss_points: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ArkLeague {
    Golden,
    Silver,
    Osiris,
    Practice,
    Custom,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArkParticipant {
    pub participated: Option<bool>,
    pub rank: usize,
    pub name: Option<String>,
    pub score: Option<f64>,
    pub occupation_score: Option<f64>,
    pub provisions_score: Option<f64>,
    pub kill_score: Option<f64>,
    pub ark_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArkHighlight {
    pub category: String,
    pub alliance_value: Option<f64>,
    pub player_name: Option<String>,
    pub player_value: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ArkReportStatus {
    Available,
    Missing,
    Ambiguous,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArkReportCoverage {
    pub status: ArkReportStatus,
    pub total: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArkReportsResponse {
    pub coverage: ArkReportCoverage,
    pub page: u64,
    pub page_size: u64,
    pub items: Vec<crate::routes::reports::battle::types::ReportListItem>,
}
