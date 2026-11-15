use serde::Serialize;

use crate::auth::Roles;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AuthMeResponse {
    pub user: CurrentUser,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CurrentUser {
    pub discord_id: String,
    pub email: String,
    pub roles: Roles,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::Role;

    #[test]
    fn me_response_exposes_effective_roles_including_implicit_user() {
        for (roles, expected) in [
            (Roles::default(), serde_json::json!(["user"])),
            (
                [Role::Admin, Role::DeveloperProgram, Role::ContentCreator, Role::Premium]
                    .into_iter()
                    .collect(),
                serde_json::json!([
                    "user",
                    "premium",
                    "content_creator",
                    "developer_program",
                    "admin"
                ]),
            ),
        ] {
            let response = AuthMeResponse {
                user: CurrentUser {
                    discord_id: "user".to_string(),
                    email: "user@example.com".to_string(),
                    roles,
                    claimed_governors: vec![],
                },
            };

            let json = serde_json::to_value(response).expect("me response");

            assert_eq!(json["user"]["roles"], expected);
        }
    }
}
