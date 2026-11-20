//! Exchanges one-time authorization codes and rotates delegated tokens.

use std::sync::Arc;

use axum::{
    Form, Json,
    extract::{State, rejection::FormRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use mongodb::bson::{DateTime, oid::ObjectId};
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;

use super::{
    Oidc, Scope, account_error, after,
    flow::registered_redirect,
    hash, opaque_token, provider, random_token, scope_string, storage_error,
    store::{AccessToken, Authorization, RefreshFamily, SpentRefreshToken, TokenHistory, grant_id},
};
use crate::{
    db::{DeveloperApp, SessionRecord},
    error::ApiError,
    state::AppState,
};

const ID_TOKEN_TTL_SECONDS: i64 = 300;
const TOKEN_TTL_SECONDS: i64 = 7 * 24 * 60 * 60;
const TOKEN_HISTORY_BATCH_SIZE: usize = 4096;

#[derive(Deserialize)]
pub(super) struct TokenForm {
    grant_type: String,
    client_id: String,
    code: Option<String>,
    redirect_uri: Option<String>,
    code_verifier: Option<String>,
    refresh_token: Option<String>,
    scope: Option<String>,
    client_secret: Option<String>,
}

#[derive(Serialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id_token: Option<String>,
    token_type: &'static str,
    expires_in: i64,
    scope: String,
}

enum TokenError {
    OAuth(&'static str),
    Internal(ApiError),
}

impl From<ApiError> for TokenError {
    fn from(error: ApiError) -> Self {
        Self::Internal(error)
    }
}

impl IntoResponse for TokenError {
    fn into_response(self) -> Response {
        match self {
            Self::OAuth(error) => {
                (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": error })))
                    .into_response()
            }
            Self::Internal(ApiError::NotFound(message)) => {
                ApiError::NotFound(message).into_response()
            }
            Self::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "server_error"
                })),
            )
                .into_response(),
        }
    }
}

fn invalid_grant() -> TokenError {
    TokenError::OAuth("invalid_grant")
}

pub(super) async fn exchange(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    form: Result<Form<TokenForm>, FormRejection>,
) -> Response {
    exchange_request(&state, &headers, form).await.into_response()
}

async fn exchange_request(
    state: &AppState,
    headers: &HeaderMap,
    form: Result<Form<TokenForm>, FormRejection>,
) -> Result<impl IntoResponse, TokenError> {
    let oidc = provider(state)?;
    let Form(form) = form.map_err(|_error| TokenError::OAuth("invalid_request"))?;
    if headers.contains_key(header::AUTHORIZATION) || form.client_secret.is_some() {
        return Err(TokenError::OAuth("invalid_client"));
    }
    let client_id = ObjectId::parse_str(&form.client_id)
        .ok()
        .filter(|id| id.to_hex() == form.client_id)
        .ok_or(TokenError::OAuth("invalid_client"))?;
    let now = DateTime::now();

    let response = match form.grant_type.as_str() {
        "authorization_code" => code_exchange(state, oidc, client_id, form, now).await?,
        "refresh_token" => refresh_exchange(state, oidc, client_id, form, now).await?,
        _ => return Err(TokenError::OAuth("unsupported_grant_type")),
    };

    Ok(([(header::PRAGMA, "no-cache")], Json(response)))
}

async fn code_exchange(
    state: &AppState,
    oidc: &Oidc,
    client_id: ObjectId,
    form: TokenForm,
    now: DateTime,
) -> Result<TokenResponse, TokenError> {
    let code =
        form.code.as_deref().filter(|value| opaque_token(value)).ok_or_else(invalid_grant)?;
    let verifier = form
        .code_verifier
        .as_deref()
        .filter(|value| valid_verifier(value))
        .ok_or_else(invalid_grant)?;
    let code_hash = hash(code);
    let stored =
        oidc.store.code(&code_hash, now).await.map_err(storage_error)?.ok_or_else(invalid_grant)?;

    if stored.authorization.client_id != client_id
        || form.redirect_uri.as_deref() != Some(&stored.redirect_uri)
        || !bool::from(hash(verifier).as_bytes().ct_eq(stored.challenge.as_bytes()))
    {
        return Err(invalid_grant());
    }
    let app = live_grant(state, oidc, &stored.authorization).await?.ok_or_else(invalid_grant)?;
    if registered_redirect(&app, &stored.redirect_uri).is_none() {
        return Err(invalid_grant());
    }
    let session =
        authorizing_session(state, &stored.authorization, now).await?.ok_or_else(invalid_grant)?;

    let id_token = oidc.config.sign(&serde_json::json!({
        "iss": oidc.origin(),
        "sub": stored.authorization.subject,
        "aud": client_id.to_hex(),
        "iat": now.timestamp_millis().div_euclid(1000),
        "exp": after(now, ID_TOKEN_TTL_SECONDS).min(session.expires_at).timestamp_millis().div_euclid(1000),
        "nonce": stored.nonce,
    })).map_err(|_error| ApiError::internal("ID token serialization failed"))?;

    if !oidc.store.consume_code(&code_hash, DateTime::now()).await.map_err(storage_error)? {
        return Err(invalid_grant());
    }

    let id = ObjectId::new();
    let (response, access) = new_pair(id, &stored.authorization.scopes, Some(id_token), now);
    let family = RefreshFamily {
        id,
        authorization: stored.authorization,
        current_hash: hash(&response.refresh_token),
        spent_hashes: Vec::new(),
        spent_refresh_tokens: Vec::new(),
        refresh_expires_at: access.expires_at,
        expires_at: access.expires_at,
        access_tokens: vec![access],
        revoked: false,
        rolling: true,
    };
    oidc.store.insert_family(&family).await.map_err(storage_error)?;

    Ok(response)
}

async fn refresh_exchange(
    state: &AppState,
    oidc: &Oidc,
    client_id: ObjectId,
    form: TokenForm,
    now: DateTime,
) -> Result<TokenResponse, TokenError> {
    let token = form.refresh_token.as_deref().ok_or_else(invalid_grant)?;
    let id = family_id(token, "r").ok_or_else(invalid_grant)?;
    let mut family =
        oidc.store.family(id).await.map_err(storage_error)?.ok_or_else(invalid_grant)?;
    let previous_expiry = family.refresh_expires_at.min(family.expires_at);
    if family.authorization.client_id != client_id || family.revoked || previous_expiry <= now {
        return Err(invalid_grant());
    }

    let current_hash = hash(token);
    if !bool::from(current_hash.as_bytes().ct_eq(family.current_hash.as_bytes())) {
        oidc.store.revoke_replayed_family(id, &current_hash, now).await.map_err(storage_error)?;
        return Err(invalid_grant());
    }
    if !live_family(state, oidc, &family, now).await? {
        return Err(invalid_grant());
    }
    let scopes = refresh_scopes(form.scope.as_deref(), &family.authorization.scopes)?;

    family.spent_refresh_tokens.extend(
        std::mem::take(&mut family.spent_hashes)
            .into_iter()
            .map(|hash| SpentRefreshToken { hash, expires_at: previous_expiry }),
    );
    if family.spent_refresh_tokens.len() >= TOKEN_HISTORY_BATCH_SIZE
        || family.access_tokens.len() >= TOKEN_HISTORY_BATCH_SIZE
    {
        // Copy only previously committed tokens before the conditional family write.
        // Concurrent refreshes can safely archive the same snapshot under this hash.
        oidc.store
            .archive_token_history(&TokenHistory {
                id: current_hash.clone(),
                family_id: id,
                spent_refresh_tokens: std::mem::take(&mut family.spent_refresh_tokens),
                access_tokens: std::mem::take(&mut family.access_tokens),
                expires_at: family.expires_at,
            })
            .await
            .map_err(storage_error)?;
    } else {
        family.spent_refresh_tokens.retain(|spent| spent.expires_at > now);
        family.access_tokens.retain(|access| access.expires_at > now);
    }

    let (response, access) = new_pair(id, &scopes, None, now);
    family.current_hash = hash(&response.refresh_token);
    family
        .spent_refresh_tokens
        .push(SpentRefreshToken { hash: current_hash.clone(), expires_at: previous_expiry });
    family.refresh_expires_at = access.expires_at;
    family.expires_at = access.expires_at;
    family.access_tokens.push(access);
    family.rolling = true;

    if !oidc.store.rotate_family(&family, &current_hash).await.map_err(storage_error)? {
        // A concurrent refresh may have spent this token after our read.
        oidc.store.revoke_replayed_family(id, &current_hash, now).await.map_err(storage_error)?;
        return Err(invalid_grant());
    }

    Ok(response)
}

fn refresh_scopes(request: Option<&str>, granted: &[Scope]) -> Result<Vec<Scope>, TokenError> {
    let Some(request) = request else {
        return Ok(granted.to_vec());
    };
    let scopes = Scope::parse(request).map_err(TokenError::OAuth)?;
    if scopes.iter().any(|scope| !granted.contains(scope)) {
        return Err(TokenError::OAuth("invalid_scope"));
    }
    Ok(scopes)
}

fn new_pair(
    id: ObjectId,
    scopes: &[Scope],
    id_token: Option<String>,
    now: DateTime,
) -> (TokenResponse, AccessToken) {
    let id = id.to_hex();
    let access_token = format!("a.{id}.{}", random_token());
    let refresh_token = format!("r.{id}.{}", random_token());
    let expiry = after(now, TOKEN_TTL_SECONDS);
    let access =
        AccessToken { hash: hash(&access_token), scopes: scopes.to_vec(), expires_at: expiry };
    let response = TokenResponse {
        access_token,
        refresh_token,
        id_token,
        token_type: "Bearer",
        expires_in: (expiry.timestamp_millis() - now.timestamp_millis()).div_euclid(1000),
        scope: scope_string(scopes),
    };

    (response, access)
}

async fn live_grant(
    state: &AppState,
    oidc: &Oidc,
    authorization: &Authorization,
) -> Result<Option<DeveloperApp>, ApiError> {
    let Some(app) = oidc.eligible_app(state.auth_store.as_ref(), authorization.client_id).await?
    else {
        return Ok(None);
    };
    let id = grant_id(&authorization.subject, authorization.client_id);
    let grant = oidc.store.grant(&id).await.map_err(storage_error)?;
    if !grant.is_some_and(|grant| {
        !grant.revoked
            && grant.generation == authorization.grant_generation
            && grant.owner_id == app.owner_id
            && grant.user_id == authorization.user_id
            && authorization.scopes.iter().all(|scope| grant.scopes.contains(scope))
    }) {
        return Ok(None);
    }

    let subject = oidc.store.subject(&authorization.user_id).await.map_err(storage_error)?;
    Ok((subject.as_deref() == Some(authorization.subject.as_str())).then_some(app))
}

async fn authorizing_session(
    state: &AppState,
    authorization: &Authorization,
    now: DateTime,
) -> Result<Option<SessionRecord>, ApiError> {
    let session = state
        .auth_store
        .active_session_by_record_id(&authorization.user_id, authorization.session_id, now)
        .await
        .map_err(account_error)?;

    Ok(session.filter(|session| {
        session.id == authorization.session_id
            && session.user_id == authorization.user_id
            && session.expires_at > now
    }))
}

async fn live_family(
    state: &AppState,
    oidc: &Oidc,
    family: &RefreshFamily,
    now: DateTime,
) -> Result<bool, ApiError> {
    if !family.rolling && authorizing_session(state, &family.authorization, now).await?.is_none() {
        return Ok(false);
    }

    Ok(live_grant(state, oidc, &family.authorization).await?.is_some())
}

/// A scoped access token accepted by a shared resource route.
pub(crate) struct AppAccess {
    token: String,
    family: ObjectId,
    client_id: ObjectId,
    pub(crate) account_id: String,
    pub(crate) subject: String,
    pub(crate) scopes: Vec<Scope>,
    pub(crate) shared_governor_memberships: Vec<ObjectId>,
}

impl AppAccess {
    pub(crate) async fn ensure_active(&self, state: &AppState) -> Result<(), ApiError> {
        let oidc = provider(state)?;
        let renewed = load_access(state, oidc, &self.token, self.family, DateTime::now()).await?;

        if renewed.is_none() {
            return Err(ApiError::unauthorized());
        }
        Ok(())
    }
}

pub(crate) async fn authorize_access(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AppAccess, ApiError> {
    let oidc = provider(state).map_err(|_error| ApiError::unauthorized())?;
    let header =
        single_header(headers, header::AUTHORIZATION).ok_or_else(ApiError::unauthorized)?;
    let (kind, token) = header.split_once(' ').ok_or_else(ApiError::unauthorized)?;
    if !kind.eq_ignore_ascii_case("bearer") {
        return Err(ApiError::unauthorized());
    }
    let family = family_id(token, "a").ok_or_else(ApiError::unauthorized)?;
    let access = load_access(state, oidc, token, family, DateTime::now())
        .await?
        .ok_or_else(ApiError::unauthorized)?;
    if !single_header(headers, header::USER_AGENT)
        .is_some_and(|agent| matches_user_agent(agent, access.client_id))
    {
        return Err(ApiError::Forbidden("invalid-app-user-agent".to_owned()));
    }
    Ok(access)
}

async fn load_access(
    state: &AppState,
    oidc: &Oidc,
    token: &str,
    id: ObjectId,
    now: DateTime,
) -> Result<Option<AppAccess>, ApiError> {
    let Some(family) = oidc.store.family(id).await.map_err(storage_error)? else {
        return Ok(None);
    };
    if family.revoked || family.expires_at <= now {
        return Ok(None);
    }
    let digest = hash(token);
    let inline = family.access_tokens.iter().find(|entry| {
        entry.expires_at > now && bool::from(digest.as_bytes().ct_eq(entry.hash.as_bytes()))
    });
    let access = match inline {
        Some(access) => Some(access.clone()),
        None => oidc.store.archived_access(id, &digest, now).await.map_err(storage_error)?,
    };
    let Some(access) = access else {
        return Ok(None);
    };
    if !live_family(state, oidc, &family, now).await? {
        return Ok(None);
    }

    Ok(Some(AppAccess {
        token: token.to_owned(),
        family: id,
        client_id: family.authorization.client_id,
        account_id: family.authorization.user_id,
        subject: family.authorization.subject,
        scopes: access.scopes,
        shared_governor_memberships: family.authorization.shared_governor_memberships,
    }))
}

fn family_id(token: &str, expected: &str) -> Option<ObjectId> {
    let (kind, rest) = token.split_once('.')?;
    let (id, secret) = rest.split_once('.')?;
    if kind != expected || !opaque_token(secret) {
        return None;
    }
    let parsed = ObjectId::parse_str(id).ok()?;
    (parsed.to_hex() == id).then_some(parsed)
}

fn valid_verifier(value: &str) -> bool {
    (43..=128).contains(&value.len())
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"-._~".contains(&byte))
}

fn matches_user_agent(value: &str, id: ObjectId) -> bool {
    let product = format!("ROKBattles/{}", id.to_hex());
    let Some(rest) = value.strip_prefix(&product) else {
        return false;
    };
    rest.is_empty()
        || rest.strip_prefix(" (").and_then(|value| value.strip_suffix(')')).is_some_and(|name| {
            (1..=100).contains(&name.len())
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b" -_./;".contains(&byte))
        })
}

fn single_header(headers: &HeaderMap, key: axum::http::HeaderName) -> Option<&str> {
    let mut values = headers.get_all(key).iter();
    let first = values.next()?.to_str().ok()?;
    values.next().is_none().then_some(first)
}

pub(crate) fn bearer_rejection(error: ApiError) -> Response {
    let challenge = match &error {
        ApiError::Unauthorized => Some("Bearer error=\"invalid_token\""),
        ApiError::Forbidden(message) if message == "insufficient_scope" => {
            Some("Bearer error=\"insufficient_scope\"")
        }
        _ => None,
    };
    let mut response = error.into_response();
    if let Some(challenge) = challenge {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static(challenge));
    }
    response
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::http::Method;
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use ring::signature::{ED25519, UnparsedPublicKey};
    use serde_json::{Value, json};

    use super::*;
    use crate::{
        oidc::test_support::{Fixture, ORIGIN, USER, VERIFIER, body, claim},
        rate_limit::RateLimitMode,
    };

    async fn issue_pair(fixture: &Fixture) -> Value {
        let code = fixture.code("openid").await;
        let response = fixture.exchange(&code, VERIFIER).await;
        assert_eq!(response.status(), StatusCode::OK);
        body(response).await
    }

    async fn refresh_at(
        fixture: &Fixture,
        token: &str,
        now: DateTime,
    ) -> Result<TokenResponse, TokenError> {
        let form = serde_json::from_value(json!({
            "grant_type": "refresh_token",
            "client_id": fixture.id,
            "refresh_token": token,
        }))
        .expect("refresh form");
        let oidc = provider(&fixture.state).expect("OIDC provider");
        let client = ObjectId::parse_str(&fixture.id).expect("client ID");
        refresh_exchange(&fixture.state, oidc, client, form, now).await
    }

    async fn access_valid_at(fixture: &Fixture, token: &str, now: DateTime) -> bool {
        let oidc = provider(&fixture.state).expect("OIDC provider");
        let id = family_id(token, "a").expect("access token family");
        load_access(&fixture.state, oidc, token, id, now).await.expect("token lookup").is_some()
    }

    async fn stored_family(fixture: &Fixture, refresh: &str) -> RefreshFamily {
        let oidc = provider(&fixture.state).expect("OIDC provider");
        let id = family_id(refresh, "r").expect("refresh token family");
        oidc.store.family(id).await.expect("family lookup").expect("token family")
    }

    #[test]
    fn access_tokens_last_seven_days_from_issuance() {
        let now = DateTime::from_millis(0);
        let (response, token) = new_pair(ObjectId::new(), &[Scope::OpenId], None, now);
        assert_eq!(response.expires_in, 604_800);
        assert_eq!(token.expires_at, after(now, 604_800));
    }

    #[test]
    fn checks_s256_pkce_vector_from_rfc7636() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert!(valid_verifier(verifier));
        assert_eq!(hash(verifier), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
        assert!(!valid_verifier("short"));
        assert!(!valid_verifier(&"é".repeat(43)));
    }

    #[test]
    fn matches_the_app_id_across_renames() {
        let id = ObjectId::new();
        let product = format!("ROKBattles/{}", id.to_hex());
        assert!(matches_user_agent(&product, id));
        assert!(matches_user_agent(&format!("{product} (Old_name)"), id));
        assert!(matches_user_agent(&format!("{product} (New name)"), id));
        assert!(!matches_user_agent(&format!("{product} (Bad)(Name)"), id));
        assert!(!matches_user_agent(&format!("{product}X"), id));
        assert!(!matches_user_agent(&product, ObjectId::new()));
    }

    #[test]
    fn accepts_optional_app_version_and_platform_in_the_user_agent() {
        let id = ObjectId::new();
        let agent = format!("ROKBattles/{} (MyApp/1.0.0; Windows/10)", id.to_hex());
        assert!(matches_user_agent(&agent, id));
    }

    #[test]
    fn refresh_can_narrow_but_cannot_add_scopes() {
        let original = [Scope::OpenId, Scope::GovernorBindsRead];
        assert_eq!(refresh_scopes(Some("openid"), &original).ok(), Some(vec![Scope::OpenId]));
        assert!(matches!(
            refresh_scopes(Some("governor.binds:write"), &original),
            Err(TokenError::OAuth("invalid_scope"))
        ));
    }

    #[tokio::test]
    async fn code_issues_signed_tokens_once_and_profiles_use_the_bearer_identity() {
        let fixture = Fixture::new().await;
        let code = fixture.code("openid profile:read").await;
        assert_eq!(
            fixture.exchange(&code, &"x".repeat(43)).await.status(),
            StatusCode::BAD_REQUEST
        );

        let (first, second) =
            tokio::join!(fixture.exchange(&code, VERIFIER), fixture.exchange(&code, VERIFIER));
        let [good, bad] =
            if first.status() == StatusCode::OK { [first, second] } else { [second, first] };
        assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
        assert_eq!(good.headers().get(header::CACHE_CONTROL).expect("cache control"), "no-store");
        let issued = body(good).await;

        let jwks = body(fixture.call(Method::GET, "/v1/auth/jwks", &[], String::new()).await).await;
        let key = jwks["keys"].as_array().and_then(|list| list.first()).expect("key");
        assert!(key.get("d").is_none());
        let public = URL_SAFE_NO_PAD.decode(claim(key, "x")).expect("public key");
        let (message, signature) = claim(&issued, "id_token").rsplit_once('.').expect("JWS");
        let signature = URL_SAFE_NO_PAD.decode(signature).expect("signature");
        UnparsedPublicKey::new(&ED25519, public)
            .verify(message.as_bytes(), &signature)
            .expect("Ed25519 signature");
        let (header, claims) = message.split_once('.').expect("claims");
        let header: Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(header).expect("header bytes"))
                .expect("header");
        let claims: Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(claims).expect("claims bytes"))
                .expect("claims");
        assert_eq!(header["alg"], "Ed25519");
        assert_eq!(header["kid"], key["kid"]);
        assert_eq!(claims["iss"], ORIGIN);
        assert_eq!(claims["aud"], fixture.id);
        assert_eq!(claims["nonce"], "test-nonce");
        assert!(claims.get("email").is_none());
        assert!(claims.get("discord_id").is_none());

        let access = claim(&issued, "access_token");
        let profile = fixture.profile(access, "sid=bob").await;
        assert_eq!(profile.status(), StatusCode::OK);
        assert_eq!(body(profile).await, json!({ "sub": claims["sub"], "discord_id": USER }));
        let session = fixture
            .call(Method::GET, "/v1/me/profile", &[("cookie", "sid=alice")], String::new())
            .await;
        assert_eq!(body(session).await, json!({ "sub": claims["sub"], "discord_id": USER }));
        assert_eq!(
            fixture.profile("invalid", "sid=alice").await.status(),
            StatusCode::UNAUTHORIZED
        );
        let missing_scope = fixture
            .call(
                Method::GET,
                "/v1/me/governor-binds",
                &[
                    ("cookie", "sid=alice"),
                    ("authorization", &format!("Bearer {access}")),
                    ("user-agent", &format!("ROKBattles/{}", fixture.id)),
                ],
                String::new(),
            )
            .await;
        assert_eq!(missing_scope.status(), StatusCode::FORBIDDEN);
        assert!(missing_scope.headers().contains_key(header::WWW_AUTHENTICATE));
    }

    #[tokio::test]
    async fn bearer_profile_and_governor_binds_have_independent_budgets_per_ip() {
        let fixture =
            Fixture::with_read_rate_limits(RateLimitMode::Cloudflare, 1, Duration::from_secs(60))
                .await;
        let code = fixture.code("openid governor.binds:read").await;
        let issued = fixture.exchange(&code, VERIFIER).await;
        assert_eq!(issued.status(), StatusCode::OK);
        let tokens = body(issued).await;
        let bearer = format!("Bearer {}", claim(&tokens, "access_token"));
        let user_agent = format!("ROKBattles/{}", fixture.id);

        // Exhaust the binds budget without running a report-store query.
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", HeaderValue::from_static("192.0.2.2"));
        provider(&fixture.state)
            .expect("OIDC provider")
            .governor_binds_read_limiter
            .check_ip(&headers)
            .expect("initial governor-binds allowance");

        for (path, ip, expected) in [
            ("/v1/me/profile", "192.0.2.1", StatusCode::OK),
            ("/v1/me/profile", "192.0.2.1", StatusCode::TOO_MANY_REQUESTS),
            ("/v1/me/governor-binds", "192.0.2.2", StatusCode::TOO_MANY_REQUESTS),
            ("/v1/me/profile", "192.0.2.2", StatusCode::OK),
        ] {
            let response = fixture
                .call(
                    Method::GET,
                    path,
                    &[
                        ("authorization", &bearer),
                        ("user-agent", &user_agent),
                        ("cf-connecting-ip", ip),
                    ],
                    String::new(),
                )
                .await;
            assert_eq!(response.status(), expected, "{path} from {ip}");
            if expected == StatusCode::TOO_MANY_REQUESTS {
                assert!(response.headers().contains_key(header::RETRY_AFTER));
            }
        }
    }

    #[tokio::test]
    async fn bearer_governor_data_reads_share_a_budget_per_ip() {
        let fixture =
            Fixture::with_read_rate_limits(RateLimitMode::Cloudflare, 1, Duration::from_secs(60))
                .await;
        let code = fixture.code("openid governor.data:read").await;
        let issued = fixture.exchange(&code, VERIFIER).await;
        assert_eq!(issued.status(), StatusCode::OK);
        let tokens = body(issued).await;
        let bearer = format!("Bearer {}", claim(&tokens, "access_token"));
        let user_agent = format!("ROKBattles/{}", fixture.id);

        // Invalid governor IDs let the handlers respond without querying MongoDB.
        for (path, ip, expected) in [
            ("/v1/me/governors/0/reports/battle", "192.0.2.1", StatusCode::BAD_REQUEST),
            ("/v1/me/governors/0/loot/barbarians", "192.0.2.1", StatusCode::TOO_MANY_REQUESTS),
            ("/v1/me/governors/0/resources", "192.0.2.2", StatusCode::BAD_REQUEST),
        ] {
            let response = fixture
                .call(
                    Method::GET,
                    path,
                    &[
                        ("authorization", &bearer),
                        ("user-agent", &user_agent),
                        ("cf-connecting-ip", ip),
                    ],
                    String::new(),
                )
                .await;
            assert_eq!(response.status(), expected, "{path} from {ip}");
        }
    }

    #[tokio::test]
    async fn expired_access_can_refresh_but_replay_ends_access() {
        let fixture = Fixture::new().await;
        let issued = issue_pair(&fixture).await;
        let access = claim(&issued, "access_token");
        let refresh = claim(&issued, "refresh_token");
        fixture.expire_access_tokens();
        assert_eq!(fixture.profile(access, "sid=alice").await.status(), StatusCode::UNAUTHORIZED);
        let form = [("grant_type", "refresh_token"), ("refresh_token", refresh)];
        let rotated = fixture.token(&form).await;
        assert_eq!(rotated.status(), StatusCode::OK);
        let rotated = body(rotated).await;
        let access = claim(&rotated, "access_token");

        assert_eq!(
            body(fixture.profile(access, "").await).await,
            json!({ "sub": format!("subject-{USER}") })
        );
        let replay = fixture.token(&form).await;
        assert_eq!(replay.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body(replay).await, json!({ "error": "invalid_grant" }));
        assert_eq!(fixture.profile(access, "").await.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn issued_app_tokens_survive_browser_logout_but_pending_codes_do_not() {
        let fixture = Fixture::new().await;
        let issued = issue_pair(&fixture).await;
        let pending_code = fixture.code("openid").await;
        let logout = fixture
            .call(Method::POST, "/v1/auth/logout", &[("cookie", "sid=alice")], String::new())
            .await;
        assert_eq!(logout.status(), StatusCode::OK);
        assert_eq!(
            fixture.profile(claim(&issued, "access_token"), "").await.status(),
            StatusCode::OK
        );
        let refresh = fixture
            .token(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", claim(&issued, "refresh_token")),
            ])
            .await;
        assert_eq!(refresh.status(), StatusCode::OK);
        assert_eq!(body(refresh).await["expires_in"], 604_800);
        assert_eq!(
            fixture.exchange(&pending_code, VERIFIER).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn refreshing_beyond_the_browser_session_restarts_the_seven_day_window() {
        let fixture = Fixture::new().await;
        let initial = issue_pair(&fixture).await;
        let initial_refresh = claim(&initial, "refresh_token");
        let initial_access = claim(&initial, "access_token");
        let initial_family = stored_family(&fixture, initial_refresh).await;
        assert_eq!(initial["expires_in"], 604_800);
        assert_eq!(initial_family.expires_at, initial_family.refresh_expires_at);
        let first_deadline = initial_family.refresh_expires_at;
        let first_rotation = after(first_deadline, -1);
        assert!(
            authorizing_session(&fixture.state, &initial_family.authorization, first_rotation)
                .await
                .expect("session lookup")
                .is_none()
        );
        assert!(access_valid_at(&fixture, initial_access, first_rotation).await);

        let first = refresh_at(&fixture, initial_refresh, first_rotation)
            .await
            .ok()
            .expect("rotation after browser expiry");
        assert_eq!(first.expires_in, 604_800);
        assert_ne!(first.refresh_token, initial_refresh);
        assert_ne!(first.access_token, initial_access);
        assert!(!access_valid_at(&fixture, initial_access, first_deadline).await);
        assert!(matches!(
            refresh_at(&fixture, initial_refresh, first_deadline).await,
            Err(TokenError::OAuth("invalid_grant"))
        ));
        assert!(access_valid_at(&fixture, &first.access_token, first_deadline).await);

        let second_deadline = after(first_rotation, TOKEN_TTL_SECONDS);
        let second_rotation = after(second_deadline, -1);
        let second = refresh_at(&fixture, &first.refresh_token, second_rotation)
            .await
            .ok()
            .expect("second rotation");
        assert_eq!(second.expires_in, 604_800);
        let family = stored_family(&fixture, &second.refresh_token).await;
        let final_deadline = after(second_rotation, TOKEN_TTL_SECONDS);
        assert_eq!(family.refresh_expires_at, final_deadline);
        assert_eq!(family.expires_at, final_deadline);
        assert_eq!(family.spent_refresh_tokens.len(), 1);
        assert!(access_valid_at(&fixture, &second.access_token, after(final_deadline, -1)).await);
        assert!(!access_valid_at(&fixture, &second.access_token, final_deadline).await);
        assert!(matches!(
            refresh_at(&fixture, &second.refresh_token, final_deadline).await,
            Err(TokenError::OAuth("invalid_grant"))
        ));
    }

    async fn mark_as_legacy(fixture: &Fixture, tokens: &Value) -> DateTime {
        let oidc = provider(&fixture.state).expect("OIDC provider");
        let mut family = stored_family(fixture, claim(tokens, "refresh_token")).await;
        let session = authorizing_session(&fixture.state, &family.authorization, DateTime::now())
            .await
            .expect("session lookup")
            .expect("session");
        let expiry = session.expires_at;
        family.refresh_expires_at = expiry;
        family.expires_at = expiry;
        family.access_tokens[0].expires_at = expiry;

        let mut document = mongodb::bson::to_document(&family).expect("token family document");
        document.remove("rolling");
        document.remove("spentRefreshTokens");
        let legacy: RefreshFamily = mongodb::bson::from_document(document).expect("legacy family");
        assert!(!legacy.rolling);
        assert!(
            oidc.store.rotate_family(&legacy, &family.current_hash).await.expect("store legacy")
        );
        expiry
    }

    #[tokio::test]
    async fn only_valid_legacy_families_upgrade_to_rolling_on_refresh() {
        let fixture = Fixture::new().await;
        let existing = issue_pair(&fixture).await;
        let existing_access = claim(&existing, "access_token");
        let existing_refresh = claim(&existing, "refresh_token");
        let expiry = mark_as_legacy(&fixture, &existing).await;
        assert!(matches!(
            refresh_at(&fixture, existing_refresh, expiry).await,
            Err(TokenError::OAuth("invalid_grant"))
        ));

        let other = issue_pair(&fixture).await;
        mark_as_legacy(&fixture, &other).await;
        let now = DateTime::now();
        assert!(access_valid_at(&fixture, existing_access, now).await);
        let upgraded = refresh_at(&fixture, existing_refresh, now)
            .await
            .ok()
            .expect("valid legacy token upgrades");
        assert_eq!(upgraded.expires_in, 604_800);
        assert!(!access_valid_at(&fixture, existing_access, expiry).await);
        assert!(access_valid_at(&fixture, &upgraded.access_token, expiry).await);

        let logout = fixture
            .call(Method::POST, "/v1/auth/logout", &[("cookie", "sid=alice")], String::new())
            .await;
        assert_eq!(logout.status(), StatusCode::OK);
        assert!(access_valid_at(&fixture, &upgraded.access_token, DateTime::now()).await);
        assert!(!access_valid_at(&fixture, claim(&other, "access_token"), DateTime::now()).await);
        assert!(matches!(
            refresh_at(&fixture, claim(&other, "refresh_token"), DateTime::now()).await,
            Err(TokenError::OAuth("invalid_grant"))
        ));
    }

    #[tokio::test]
    async fn full_token_history_is_archived_without_limiting_refresh() {
        let fixture = Fixture::new().await;
        let initial = issue_pair(&fixture).await;
        let refresh = claim(&initial, "refresh_token");
        let access = claim(&initial, "access_token");
        let mut family = stored_family(&fixture, refresh).await;
        let now = DateTime::now();
        let expires_soon = after(now, 1);
        let expires_soon_token = format!("r.{}.{}", family.id.to_hex(), random_token());
        let unexpired_spent_token = format!("r.{}.{}", family.id.to_hex(), random_token());
        family.spent_refresh_tokens = vec![
            SpentRefreshToken {
                hash: hash(&unexpired_spent_token),
                expires_at: family.expires_at
            };
            TOKEN_HISTORY_BATCH_SIZE - 1
        ];
        family
            .spent_refresh_tokens
            .push(SpentRefreshToken { hash: hash(&expires_soon_token), expires_at: expires_soon });
        let oidc = provider(&fixture.state).expect("OIDC provider");
        assert!(
            oidc.store.rotate_family(&family, &family.current_hash).await.expect("seed history")
        );

        let renewed =
            refresh_at(&fixture, refresh, now).await.ok().expect("refresh with full history");
        let updated = stored_family(&fixture, &renewed.refresh_token).await;
        assert_eq!(updated.spent_refresh_tokens.len(), 1);
        assert_eq!(updated.access_tokens.len(), 1);
        assert!(access_valid_at(&fixture, access, now).await);
        assert!(!access_valid_at(&fixture, access, family.expires_at).await);

        assert!(matches!(
            refresh_at(&fixture, &expires_soon_token, expires_soon).await,
            Err(TokenError::OAuth("invalid_grant"))
        ));
        assert!(access_valid_at(&fixture, &renewed.access_token, expires_soon).await);
        assert!(matches!(
            refresh_at(&fixture, &unexpired_spent_token, expires_soon).await,
            Err(TokenError::OAuth("invalid_grant"))
        ));
        assert!(!access_valid_at(&fixture, &renewed.access_token, expires_soon).await);
    }

    #[tokio::test]
    async fn a_rolling_token_requires_the_grant_to_match_an_existing_account() {
        let fixture = Fixture::new().await;
        let tokens = issue_pair(&fixture).await;
        fixture.remove_account_subject();

        assert_eq!(
            fixture.profile(claim(&tokens, "access_token"), "").await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert!(matches!(
            refresh_at(&fixture, claim(&tokens, "refresh_token"), DateTime::now()).await,
            Err(TokenError::OAuth("invalid_grant"))
        ));
    }

    #[tokio::test]
    async fn concurrent_refresh_spends_the_token_once_and_revokes_replayed_family() {
        let fixture = Fixture::new().await;
        let code = fixture.code("openid profile:read").await;
        let issued = body(fixture.exchange(&code, VERIFIER).await).await;
        let refresh = claim(&issued, "refresh_token");
        let params = [("grant_type", "refresh_token"), ("refresh_token", refresh)];
        let (first, second) = tokio::join!(fixture.token(&params), fixture.token(&params));
        let [rotated, replay] =
            if first.status() == StatusCode::OK { [first, second] } else { [second, first] };

        assert_eq!(rotated.status(), StatusCode::OK);
        assert_eq!(body(replay).await, json!({ "error": "invalid_grant" }));
        let rotated = body(rotated).await;
        assert_eq!(
            fixture.profile(claim(&rotated, "access_token"), "").await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn an_unknown_refresh_token_does_not_revoke_an_active_family() {
        let fixture = Fixture::new().await;
        let code = fixture.code("openid").await;
        let issued = body(fixture.exchange(&code, VERIFIER).await).await;
        let refresh = claim(&issued, "refresh_token");
        let (prefix, _) = refresh.rsplit_once('.').expect("credential delimiter");
        let random = format!("{prefix}.{}", random_token());
        let rejected =
            fixture.token(&[("grant_type", "refresh_token"), ("refresh_token", &random)]).await;
        assert_eq!(body(rejected).await, json!({ "error": "invalid_grant" }));

        let valid =
            fixture.token(&[("grant_type", "refresh_token"), ("refresh_token", refresh)]).await;
        assert_eq!(valid.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn expanding_consent_does_not_widen_refresh_tokens_already_issued() {
        let fixture = Fixture::new().await;
        let code = fixture.code("openid").await;
        let initial = body(fixture.exchange(&code, VERIFIER).await).await;

        let wider = fixture.code("openid profile:read").await;
        assert_eq!(fixture.exchange(&wider, VERIFIER).await.status(), StatusCode::OK);

        let refresh = claim(&initial, "refresh_token");
        let invalid = fixture
            .token(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh),
                ("scope", "openid profile:read"),
            ])
            .await;
        assert_eq!(body(invalid).await, json!({ "error": "invalid_scope" }));

        let original =
            fixture.token(&[("grant_type", "refresh_token"), ("refresh_token", refresh)]).await;
        let original = body(original).await;
        assert_eq!(original["scope"], "openid");
        let profile = fixture.profile(claim(&original, "access_token"), "").await;
        assert_eq!(body(profile).await, json!({ "sub": format!("subject-{USER}") }));
    }
}
