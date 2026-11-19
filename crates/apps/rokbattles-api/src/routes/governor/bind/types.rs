use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub(super) struct BindGovernorResponse {
    pub claim: ClaimedGovernor,
}

/// A governor available to the caller through a primary bind or a share.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClaimedGovernor {
    pub governor_id: i64,
    pub governor_name: Option<String>,
    pub governor_avatar: Option<String>,
    pub default: bool,
    pub is_shared: bool,
    /// Recipient Discord IDs exposed to the primary holder's session.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub shared_with: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BindSuggestion {
    pub governor_id: i64,
    pub governor_name: Option<String>,
    pub governor_avatar: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BindSelection {
    pub governor_ids: Vec<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShareSelection {
    pub governor_ids: Vec<i64>,
    pub discord_id: String,
}
