use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RefreshBindsResponse {
    pub governors_seen: usize,
    pub governors_refreshed: usize,
    pub claims_matched: u64,
    pub claims_updated: u64,
}

/// Counters captured while refreshing binds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct RefreshBindsStats {
    pub governors_seen: usize,
    pub governors_refreshed: usize,
    pub claims_matched: u64,
    pub claims_updated: u64,
}

impl From<RefreshBindsStats> for RefreshBindsResponse {
    fn from(value: RefreshBindsStats) -> Self {
        Self {
            governors_seen: value.governors_seen,
            governors_refreshed: value.governors_refreshed,
            claims_matched: value.claims_matched,
            claims_updated: value.claims_updated,
        }
    }
}
