use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AuthMeResponse {
    pub user: CurrentUser,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CurrentUser {
    pub discord_id: String,
    pub username: String,
    pub global_name: Option<String>,
    pub email: String,
    pub avatar: Option<String>,
    pub claimed_governors: Vec<ClaimedGovernor>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ClaimedGovernor {
    pub governor_id: i64,
    pub governor_name: Option<String>,
    pub governor_avatar: Option<String>,
    pub default: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct LogoutResponse {
    pub success: bool,
}
