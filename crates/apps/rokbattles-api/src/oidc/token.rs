//! Exchanges one-time authorization codes and rotates session-bound tokens.

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
    store::{AccessToken, Authorization, RefreshFamily, grant_id},
};
use crate::{
    db::{DeveloperApp, SessionRecord},
    error::ApiError,
    state::AppState,
};

const ID_TOKEN_TTL_SECONDS: i64 = 300;
const ACCESS_TTL_SECONDS: i64 = 7 * 24 * 60 * 60;
const REFRESH_IDLE_SECONDS: i64 = 7 * 24 * 60 * 60;
// Keep each family's replay history bounded, even when a client refreshes continuously.
const MAX_ROTATIONS: usize = 4096;

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
    let (session, app) = live_authorization(state, oidc, &stored.authorization, now)
        .await?
        .ok_or_else(invalid_grant)?;
    if registered_redirect(&app, &stored.redirect_uri).is_none() {
        return Err(invalid_grant());
    }

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
    let (response, access) =
        new_pair(id, &stored.authorization.scopes, Some(id_token), now, session.expires_at);
    let family = RefreshFamily {
        id,
        authorization: stored.authorization,
        current_hash: hash(&response.refresh_token),
        spent_hashes: Vec::new(),
        access_tokens: vec![access],
        refresh_expires_at: after(now, REFRESH_IDLE_SECONDS).min(session.expires_at),
        expires_at: session.expires_at,
        revoked: false,
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
    if family.authorization.client_id != client_id
        || family.revoked
        || family.expires_at <= now
        || family.refresh_expires_at <= now
    {
        return Err(invalid_grant());
    }

    let current_hash = hash(token);
    if !bool::from(current_hash.as_bytes().ct_eq(family.current_hash.as_bytes())) {
        oidc.store.revoke_replayed_family(id, &current_hash).await.map_err(storage_error)?;
        return Err(invalid_grant());
    }
    if family.spent_hashes.len() >= MAX_ROTATIONS {
        return Err(invalid_grant());
    }

    let (session, _) = live_authorization(state, oidc, &family.authorization, now)
        .await?
        .ok_or_else(invalid_grant)?;
    let scopes = refresh_scopes(form.scope.as_deref(), &family.authorization.scopes)?;
    let (response, access) = new_pair(id, &scopes, None, now, session.expires_at);
    family.current_hash = hash(&response.refresh_token);
    family.spent_hashes.push(current_hash.clone());
    family.access_tokens.retain(|access| access.expires_at > now);
    family.access_tokens.push(access);
    family.refresh_expires_at = after(now, REFRESH_IDLE_SECONDS).min(family.expires_at);

    if !oidc.store.rotate_family(&family, &current_hash).await.map_err(storage_error)? {
        // A concurrent refresh may have spent this token after our read.
        oidc.store.revoke_replayed_family(id, &current_hash).await.map_err(storage_error)?;
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
    session_expiry: DateTime,
) -> (TokenResponse, AccessToken) {
    let id = id.to_hex();
    let access_token = format!("a.{id}.{}", random_token());
    let refresh_token = format!("r.{id}.{}", random_token());
    let expiry = after(now, ACCESS_TTL_SECONDS).min(session_expiry);
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

async fn live_authorization(
    state: &AppState,
    oidc: &Oidc,
    authorization: &Authorization,
    now: DateTime,
) -> Result<Option<(SessionRecord, DeveloperApp)>, ApiError> {
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

    let session = state
        .auth_store
        .active_session_by_record_id(&authorization.user_id, authorization.session_id, now)
        .await
        .map_err(account_error)?;

    Ok(session
        .filter(|session| {
            session.id == authorization.session_id
                && session.user_id == authorization.user_id
                && session.expires_at > now
        })
        .map(|session| (session, app)))
}

/// A scoped access token accepted by a shared resource route.
pub(crate) struct AppAccess {
    token: String,
    family: ObjectId,
    pub(crate) client_id: ObjectId,
    pub(crate) account_id: String,
    pub(crate) subject: String,
    pub(crate) scopes: Vec<Scope>,
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
    let Some(access) = family.access_tokens.into_iter().find(|entry| {
        entry.expires_at > now && bool::from(digest.as_bytes().ct_eq(entry.hash.as_bytes()))
    }) else {
        return Ok(None);
    };
    if live_authorization(state, oidc, &family.authorization, now).await?.is_none() {
        return Ok(None);
    }

    Ok(Some(AppAccess {
        token: token.to_owned(),
        family: id,
        client_id: family.authorization.client_id,
        account_id: family.authorization.user_id,
        subject: family.authorization.subject,
        scopes: access.scopes,
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
    use axum::http::Method;
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use ring::signature::{ED25519, UnparsedPublicKey};
    use serde_json::{Value, json};

    use super::*;
    use crate::oidc::test_support::{Fixture, ORIGIN, USER, VERIFIER, body, claim};

    #[test]
    fn access_tokens_last_up_to_seven_days_within_the_session() {
        let now = DateTime::from_millis(0);
        let id = ObjectId::new();
        let long_session = after(now, 8 * 24 * 60 * 60);
        let (response, token) = new_pair(id, &[Scope::OpenId], None, now, long_session);
        assert_eq!(response.expires_in, 604_800);
        assert_eq!(token.expires_at, after(now, 604_800));

        let short_session = after(now, 600);
        let (response, token) = new_pair(id, &[Scope::OpenId], None, now, short_session);
        assert_eq!(response.expires_in, 600);
        assert_eq!(token.expires_at, short_session);
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
    async fn expired_access_can_refresh_but_replay_and_session_logout_end_access() {
        let fixture = Fixture::new().await;
        let code = fixture.code("openid").await;
        let issued = body(fixture.exchange(&code, VERIFIER).await).await;
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

        let code = fixture.code("openid").await;
        let issued = body(fixture.exchange(&code, VERIFIER).await).await;
        let logout = fixture
            .call(Method::POST, "/v1/auth/logout", &[("cookie", "sid=alice")], String::new())
            .await;
        assert_eq!(logout.status(), StatusCode::OK);
        assert_eq!(
            fixture.profile(claim(&issued, "access_token"), "").await.status(),
            StatusCode::UNAUTHORIZED
        );
        let refresh = fixture
            .token(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", claim(&issued, "refresh_token")),
            ])
            .await;
        assert_eq!(refresh.status(), StatusCode::BAD_REQUEST);
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
