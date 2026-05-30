use serde::Serialize;

/// Response body for successful governor bind claims.
#[derive(Debug, Serialize)]
pub(super) struct BindGovernorResponse {
    pub claim: ClaimedGovernor,
}

/// Claimed governor payload returned to clients.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ClaimedGovernor {
    pub governor_id: i64,
    pub governor_name: Option<String>,
    pub governor_avatar: Option<String>,
    pub default: bool,
}
