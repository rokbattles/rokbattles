use serde_json::Value;

use crate::{
    auth::{AuthSession, BoundSession, Credentials},
    batcher::KingdomMemberBatcher,
    config::{
        CLIENT_ID, DEFAULT_LANGUAGE, GAME_TOOLS_API, GAME_TOOLS_APP_ID, GAME_TOOLS_ORIGIN,
        PASSPORT_API, PASSPORT_ORIGIN, PLATFORM_API, RokGtConfig, USER_AGENT,
    },
    date::current_member_date_range,
    error::RokGtError,
    models::{KingdomMember, MemberDateRange, Role},
    parse::{parse_member_records, parse_server_ids, read_api_data},
    util::{client_info, signed_passport_url},
};

/// ROK Game Tools API client.
#[derive(Debug, Clone)]
pub struct RokGtClient {
    http: reqwest::Client,
    config: RokGtConfig,
}

impl RokGtClient {
    /// Use the production endpoints with the provided signing keys.
    pub fn new(config: RokGtConfig) -> Result<Self, RokGtError> {
        let http =
            reqwest::Client::builder().user_agent(USER_AGENT).timeout(config.timeout).build()?;
        Ok(Self { http, config })
    }

    /// Log in through Lilith Passport.
    pub(crate) async fn authenticate(
        &self,
        credentials: &Credentials,
    ) -> Result<AuthSession, RokGtError> {
        let pup_token = self.fetch_pup_token(credentials).await?;
        let url = format!("{PASSPORT_API}/api/v2/passport-login/password");
        let payload = serde_json::json!({
            "pup_token": pup_token,
            "client_id": CLIENT_ID,
            "username": credentials.email(),
            "password": credentials.password(),
            "account_type": 0,
            "login_free": false,
        });
        let response = self
            .http
            .post(url)
            .header("accept", "application/json, text/plain, */*")
            .header("content-type", "application/json")
            .header("x-client-info", client_info(&credentials.visitor_id()))
            .header("origin", PASSPORT_ORIGIN)
            .header("referer", format!("{PASSPORT_ORIGIN}/"))
            .json(&payload)
            .send()
            .await?;

        let data = read_api_data(response).await?;
        let token = data
            .get("jwt_token")
            .and_then(Value::as_str)
            .or_else(|| data.get("token").and_then(Value::as_str))
            .or_else(|| data.get("access_token").and_then(Value::as_str))
            .ok_or(RokGtError::MissingField("jwt_token"))?;

        Ok(AuthSession::from_pauthorization(token))
    }

    /// Load the account's available roles.
    pub(crate) async fn fetch_roles(&self, session: &AuthSession) -> Result<Vec<Role>, RokGtError> {
        let url = format!("{GAME_TOOLS_API}/api/lilith/roles");
        let response = self
            .http
            .get(url)
            .header("accept", "application/json, text/plain, */*")
            .header("lang", DEFAULT_LANGUAGE)
            .header("pauthorization", session.bearer())
            .header("origin", GAME_TOOLS_ORIGIN)
            .header("referer", format!("{GAME_TOOLS_ORIGIN}/"))
            .send()
            .await?;
        let data = read_api_data(response).await?;
        let roles_value = data
            .get("roles_list")
            .or_else(|| data.get("list"))
            .or_else(|| data.get("roles"))
            .cloned()
            .unwrap_or(Value::Array(Vec::new()));
        serde_json::from_value(roles_value).map_err(RokGtError::Json)
    }

    /// Bind a role and return the platform session used by data endpoints.
    pub(crate) async fn bind_role(
        &self,
        session: &AuthSession,
        role: &Role,
    ) -> Result<BoundSession, RokGtError> {
        let url = format!("{GAME_TOOLS_API}/api/lilith/bind_role");
        let payload = serde_json::json!({
            "app_id": role.app_id,
            "app_uid": role.app_uid,
            "uid": role.uid,
            "svr_id": role.svr_id,
        });
        let response = self
            .http
            .post(url)
            .header("accept", "application/json, text/plain, */*")
            .header("content-type", "application/json")
            .header("lang", DEFAULT_LANGUAGE)
            .header("pauthorization", session.bearer())
            .header("origin", GAME_TOOLS_ORIGIN)
            .header("referer", format!("{GAME_TOOLS_ORIGIN}/"))
            .json(&payload)
            .send()
            .await?;

        let data = read_api_data(response).await?;
        let token = data
            .get("access_token")
            .and_then(Value::as_str)
            .ok_or(RokGtError::MissingField("access_token"))?;
        Ok(BoundSession::from_tokens(session.clone(), token, role.clone()))
    }

    /// Log in and bind the highest power role.
    pub(crate) async fn authenticate_and_bind_default_role(
        &self,
        credentials: &Credentials,
    ) -> Result<BoundSession, RokGtError> {
        let session = self.authenticate(credentials).await?;
        let roles = self.fetch_roles(&session).await?;
        let role = pick_default_role(&roles).ok_or(RokGtError::NoRoles)?;
        self.bind_role(&session, role).await
    }

    /// Load the kingdom IDs exposed by `latestServerIds`.
    pub(crate) async fn fetch_available_server_ids(
        &self,
        session: &BoundSession,
    ) -> Result<Vec<u32>, RokGtError> {
        let url = format!("{PLATFORM_API}/api/latestServerIds");
        let response = self
            .http
            .get(url)
            .header("accept", "application/json, text/plain, */*")
            .header("lang", DEFAULT_LANGUAGE)
            .header("pauthorization", session.pauth_bearer())
            .header("bauthorization", session.bauth_bearer())
            .header("origin", GAME_TOOLS_ORIGIN)
            .header("referer", format!("{GAME_TOOLS_ORIGIN}/"))
            .send()
            .await?;
        let data = read_api_data(response).await?;
        let mut server_ids = parse_server_ids(&data)?;
        server_ids.sort_unstable();
        server_ids.dedup();
        Ok(server_ids)
    }

    /// Load member records for one kingdom.
    pub(crate) async fn fetch_kingdom_members(
        &self,
        session: &BoundSession,
        server_id: u32,
        range: &MemberDateRange,
    ) -> Result<Vec<KingdomMember>, RokGtError> {
        let url = format!("{PLATFORM_API}/api/kindomMember");
        let response = self
            .http
            .get(url)
            .header("accept", "application/json, text/plain, */*")
            .header("lang", DEFAULT_LANGUAGE)
            .header("pauthorization", session.pauth_bearer())
            .header("bauthorization", session.bauth_bearer())
            .header("origin", GAME_TOOLS_ORIGIN)
            .header("referer", format!("{GAME_TOOLS_ORIGIN}/"))
            .query(&[
                ("start", range.start.as_str()),
                ("end", range.end.as_str()),
                ("search", ""),
                ("server_id", &server_id.to_string()),
            ])
            .send()
            .await?;
        let data = read_api_data(response).await?;
        parse_member_records(server_id, &data)
    }

    /// Create a batch fetcher for every available kingdom.
    ///
    /// The batcher keeps the credentials long enough to retry once after an auth failure.
    pub async fn member_batches_with_credentials(
        &self,
        credentials: Credentials,
    ) -> Result<KingdomMemberBatcher<'_>, RokGtError> {
        let session = self.authenticate_and_bind_default_role(&credentials).await?;
        let server_ids = self.fetch_available_server_ids(&session).await?;
        let range = current_member_date_range()?;
        Ok(KingdomMemberBatcher {
            client: self,
            session,
            credentials: Some(credentials),
            server_ids,
            next_index: 0,
            batch_size: crate::config::default_batch_size(),
            range,
        })
    }

    async fn fetch_pup_token(&self, credentials: &Credentials) -> Result<String, RokGtError> {
        let visitor_id = credentials.visitor_id();
        let pup_url = signed_passport_url(
            PASSPORT_API,
            &format!("/api/v2/pup/{CLIENT_ID}"),
            &self.config.access_key,
            &self.config.secret_key,
        )?;
        self.http
            .get(pup_url)
            .header("accept", "application/json, text/plain, */*")
            .header("x-client-info", client_info(&visitor_id))
            .header("referer", format!("{PASSPORT_ORIGIN}/"))
            .send()
            .await?
            .error_for_status()?;

        let captcha_url = signed_passport_url(
            PASSPORT_API,
            "/api/v2/passport-login/captcha",
            &self.config.access_key,
            &self.config.secret_key,
        )?;
        let response = self
            .http
            .get(captcha_url)
            .header("accept", "application/json, text/plain, */*")
            .header("x-client-info", client_info(&visitor_id))
            .header("referer", format!("{PASSPORT_ORIGIN}/"))
            .send()
            .await?;
        let data = read_api_data(response).await?;
        data.get("pup_token")
            .and_then(Value::as_str)
            .or_else(|| data.get("token").and_then(Value::as_str))
            .map(ToString::to_string)
            .ok_or(RokGtError::MissingField("pup_token"))
    }
}

fn pick_default_role(roles: &[Role]) -> Option<&Role> {
    roles
        .iter()
        .filter(|role| role.app_id == GAME_TOOLS_APP_ID)
        .max_by_key(|role| role.power.unwrap_or(0))
        .or_else(|| roles.iter().max_by_key(|role| role.power.unwrap_or(0)))
}
