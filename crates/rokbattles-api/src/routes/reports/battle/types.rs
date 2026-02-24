use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportsResponse {
    pub items: Vec<ReportListItem>,
    pub next_after: Option<String>,
    pub previous_before: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportListItem {
    pub mail_id: String,
    pub time_start: i64,
    pub time_end: i64,
    pub sender: ReportListParticipant,
    pub opponent: ReportListParticipant,
    pub battles: i64,
    pub kill_count: i64,
    pub trade_percent: i64,
    pub summary: ReportSummary,
    pub timeline: ReportTimeline,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportListParticipant {
    pub primary_commander_id: i64,
    pub secondary_commander_id: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportSummary {
    pub sender: ReportSummaryEntry,
    pub opponent: ReportSummaryEntry,
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportSummaryEntry {
    pub troop_units: i64,
    pub dead: i64,
    pub severely_wounded: i64,
    pub slightly_wounded: i64,
    pub remaining: i64,
    pub kill_points: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportTimeline {
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub sampling: Vec<TimelineSample>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TimelineSample {
    pub tick: i64,
    pub count: i64,
}

#[derive(Debug)]
pub(crate) struct ReportRowWithCursor {
    pub mail_time: i64,
    pub item: ReportListItem,
}
