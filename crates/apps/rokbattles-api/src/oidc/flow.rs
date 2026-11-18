//! Validates authorization requests and binds browser consent to their session.

use std::sync::Arc;

use axum::{
    Form, Json,
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, header},
    response::{IntoResponse, Redirect, Response},
};
use mongodb::bson::{DateTime, oid::ObjectId};
use reqwest::Url;
use serde::{Deserialize, Serialize};

use super::{
    Oidc, Scope, ScopeInfo, account_error, after, hash, opaque_token, provider, random_token,
    same_origin, storage_error,
    store::{Authorization, BrowserBinding, Code, Grant, Request, grant_id},
};
use crate::{
    auth::{AuthenticatedSession, extract_cookie_value},
    db::DeveloperApp,
    error::ApiError,
    routes::auth::{begin_discord_login, build_clear_session_cookie},
    state::AppState,
};

pub(crate) const BROWSER_COOKIE: &str = "__Host-oidc-browser";
const REQUEST_TTL_SECONDS: i64 = 300;
const CODE_TTL_SECONDS: i64 = 120;

#[derive(Deserialize)]
pub(super) struct AuthorizeQuery {
    client_id: String,
    redirect_uri: String,
    response_type: Option<String>,
    scope: Option<String>,
    state: Option<String>,
    nonce: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    response_mode: Option<String>,
    prompt: Option<String>,
    request: Option<String>,
    request_uri: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct Handle {
    request: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Decision {
    Allow,
    Deny,
    SwitchAccount,
}

#[derive(Deserialize)]
pub(super) struct DecisionForm {
    request: String,
    decision: Decision,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConsentResponse {
    name: String,
    description: Option<String>,
    terms_url: Option<String>,
    privacy_url: Option<String>,
    masked_email: String,
    permissions: Vec<ScopeInfo>,
    can_add_and_read: bool,
}

#[derive(Serialize)]
struct ConsentEmail {
    email: String,
}

pub(super) async fn authorize(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AuthorizeQuery>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let oidc = provider(&state)?;
    let client_id = ObjectId::parse_str(&query.client_id)
        .ok()
        .filter(|id| id.to_hex() == query.client_id)
        .ok_or_else(invalid_callback)?;
    let app = oidc
        .eligible_app(state.auth_store.as_ref(), client_id)
        .await?
        .ok_or_else(invalid_callback)?;
    let redirect = registered_redirect(&app, &query.redirect_uri).ok_or_else(invalid_callback)?;

    let scopes = match validate_query(&query) {
        Ok(scopes) => scopes,
        Err(error) => return Ok(callback(redirect, query.valid_state(), "error", error)),
    };

    let session =
        match AuthenticatedSession::authenticate(&headers, state.auth_store.as_ref()).await {
            Ok(session) => Some(session),
            Err(ApiError::Unauthorized) => None,
            Err(error) => return Err(error),
        };
    if query.prompt.as_deref() == Some("none") {
        let error = if session.is_some() { "consent_required" } else { "login_required" };
        return Ok(callback(redirect, query.valid_state(), "error", error));
    }

    let browser = browser_value(&headers).unwrap_or_else(random_token);
    let mut request = Request {
        id: random_token(),
        browser_hash: hash(&browser),
        client_id,
        redirect_uri: query.redirect_uri,
        state: query.state.ok_or_else(invalid_request)?,
        nonce: query.nonce.ok_or_else(invalid_request)?,
        challenge: query.code_challenge.ok_or_else(invalid_request)?,
        scopes,
        binding: None,
        expires_at: after(DateTime::now(), REQUEST_TTL_SECONDS),
    };

    let response = if let Some(session) = session {
        request.binding = Some(binding_for(oidc, &request, &session).await?);
        oidc.store.insert_request(&request).await.map_err(storage_error)?;
        Redirect::to(&resume_path(&request.id)).into_response()
    } else {
        oidc.store.insert_request(&request).await.map_err(storage_error)?;
        let url = begin_discord_login(&state, None, Some(request.id)).await?;
        Redirect::temporary(url.as_str()).into_response()
    };

    with_browser_cookie(response, &browser)
}

pub(super) async fn resume(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Query(handle): Query<Handle>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let oidc = provider(&state)?;
    let now = DateTime::now();
    let browser_hash = browser_hash(&headers)?;
    let request = load_request(oidc, &handle.request, &browser_hash, now).await?;
    let app = oidc
        .eligible_app(state.auth_store.as_ref(), request.client_id)
        .await?
        .ok_or_else(invalid_callback)?;
    registered_redirect(&app, &request.redirect_uri).ok_or_else(invalid_callback)?;

    let _operation = state.account_operations.lock(&session.user.discord_id).await;
    session.ensure_active(state.auth_store.as_ref()).await?;
    let binding = match &request.binding {
        Some(binding) if binding.session_id == session.session.id => binding.clone(),
        Some(_) => return Err(invalid_request()),
        None => binding_for(oidc, &request, &session).await?,
    };
    let request = oidc
        .store
        .bind_request(&request.id, &browser_hash, &binding, now)
        .await
        .map_err(storage_error)?
        .ok_or_else(invalid_request)?;

    Ok(Redirect::to(&format!("/oauth/consent?request={}", request.id)).into_response())
}

pub(super) async fn consent(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Query(handle): Query<Handle>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (request, app) = consent_request(&state, &handle, &headers, &session).await?;

    Ok(Json(ConsentResponse {
        name: app.settings.name,
        description: app.settings.description,
        terms_url: app.settings.terms_of_service_url,
        privacy_url: app.settings.privacy_policy_url,
        masked_email: mask_email(&session.user.email),
        permissions: request.scopes.iter().copied().map(Scope::info).collect(),
        can_add_and_read: request.scopes.contains(&Scope::GovernorBindsWrite)
            && request.scopes.contains(&Scope::GovernorDataRead),
    }))
}

pub(super) async fn consent_email(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Query(handle): Query<Handle>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    consent_request(&state, &handle, &headers, &session).await?;

    Ok(Json(ConsentEmail { email: session.user.email }))
}

async fn consent_request(
    state: &AppState,
    handle: &Handle,
    headers: &HeaderMap,
    session: &AuthenticatedSession,
) -> Result<(Request, DeveloperApp), ApiError> {
    let oidc = provider(state)?;
    let request = bound_request(oidc, &handle.request, headers, session, DateTime::now()).await?;
    let app = oidc
        .eligible_app(state.auth_store.as_ref(), request.client_id)
        .await?
        .ok_or_else(invalid_callback)?;
    registered_redirect(&app, &request.redirect_uri).ok_or_else(invalid_callback)?;

    Ok((request, app))
}

fn mask_email(email: &str) -> String {
    if let Some((local, domain)) = email.rsplit_once('@')
        && !domain.is_empty()
    {
        let mut characters = local.chars();
        if let Some(first) = characters.next() {
            let mask = "*".repeat(characters.count());
            return format!("{first}{mask}@{domain}");
        }
    }

    "•••".to_owned()
}

pub(super) async fn decision(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    headers: HeaderMap,
    Form(form): Form<DecisionForm>,
) -> Result<Response, ApiError> {
    let oidc = provider(&state)?;
    same_origin(&headers, oidc.origin())?;

    let _operation = state.account_operations.lock(&session.user.discord_id).await;
    session.ensure_active(state.auth_store.as_ref()).await?;
    let now = DateTime::now();
    let request = bound_request(oidc, &form.request, &headers, &session, now).await?;
    let app = oidc
        .eligible_app(state.auth_store.as_ref(), request.client_id)
        .await?
        .ok_or_else(invalid_callback)?;
    let redirect = registered_redirect(&app, &request.redirect_uri).ok_or_else(invalid_callback)?;
    consume(oidc, &request, &session).await?;
    match form.decision {
        Decision::Allow => {
            let grant = approve(oidc, &request, &session, &app, now).await?;
            issue_code(oidc, &request, &session, &grant, redirect, now).await
        }
        Decision::Deny => Ok(callback(redirect, Some(&request.state), "error", "access_denied")),
        Decision::SwitchAccount => {
            let browser = browser_value(&headers).ok_or_else(invalid_request)?;
            switch_account(&state, oidc, request, &browser, &session).await
        }
    }
}

async fn switch_account(
    state: &AppState,
    oidc: &Oidc,
    request: Request,
    browser: &str,
    session: &AuthenticatedSession,
) -> Result<Response, ApiError> {
    let request = Request {
        id: random_token(),
        binding: None,
        expires_at: after(DateTime::now(), REQUEST_TTL_SECONDS),
        ..request
    };
    oidc.store.insert_request(&request).await.map_err(storage_error)?;
    let mut url = begin_discord_login(state, None, Some(request.id)).await?;
    url.query_pairs_mut().append_pair("prompt", "consent");

    state.auth_store.delete_session_by_id(&session.sid).await.map_err(account_error)?;

    // Follow the redirect with a GET so the consent form stays on this origin.
    let mut response = Redirect::to(url.as_str()).into_response();
    let clear_cookie = HeaderValue::from_str(&build_clear_session_cookie())
        .map_err(|_error| ApiError::internal("invalid session cookie"))?;
    response.headers_mut().append(header::SET_COOKIE, clear_cookie);

    with_browser_cookie(response, browser)
}

async fn approve(
    oidc: &Oidc,
    request: &Request,
    session: &AuthenticatedSession,
    app: &DeveloperApp,
    now: DateTime,
) -> Result<Grant, ApiError> {
    let binding = request.binding.as_ref().ok_or_else(invalid_request)?;
    let id = grant_id(&binding.subject, app.id);
    let previous = oidc.store.grant(&id).await.map_err(storage_error)?;
    if previous.as_ref().map(|grant| grant.version) != binding.grant_version {
        return Err(stale_consent());
    }
    let reusable =
        previous.as_ref().filter(|grant| !grant.revoked && grant.owner_id == app.owner_id);

    let mut scopes = request.scopes.clone();
    if let Some(grant) = reusable {
        scopes.extend_from_slice(&grant.scopes);
        scopes.sort_unstable();
        scopes.dedup();
    }
    let next = Grant {
        id,
        generation: reusable.map_or_else(ObjectId::new, |grant| grant.generation),
        version: ObjectId::new(),
        client_id: app.id,
        owner_id: app.owner_id.clone(),
        user_id: session.user.discord_id.clone(),
        scopes,
        authorized_on: reusable.map_or(now, |grant| grant.authorized_on),
        revoked: false,
    };
    if !oidc.store.save_grant(&next, binding.grant_version).await.map_err(storage_error)? {
        return Err(stale_consent());
    }
    Ok(next)
}

async fn issue_code(
    oidc: &Oidc,
    request: &Request,
    session: &AuthenticatedSession,
    grant: &Grant,
    redirect: Url,
    now: DateTime,
) -> Result<Response, ApiError> {
    let binding = request.binding.as_ref().ok_or_else(invalid_request)?;
    let code = random_token();
    oidc.store
        .insert_code(&Code {
            hash: hash(&code),
            authorization: Authorization {
                client_id: request.client_id,
                user_id: session.user.discord_id.clone(),
                subject: binding.subject.clone(),
                session_id: binding.session_id,
                grant_generation: grant.generation,
                scopes: request.scopes.clone(),
            },
            redirect_uri: request.redirect_uri.clone(),
            challenge: request.challenge.clone(),
            nonce: request.nonce.clone(),
            expires_at: after(now, CODE_TTL_SECONDS),
        })
        .await
        .map_err(storage_error)?;

    Ok(callback(redirect, Some(&request.state), "code", &code))
}

async fn binding_for(
    oidc: &Oidc,
    request: &Request,
    session: &AuthenticatedSession,
) -> Result<BrowserBinding, ApiError> {
    let subject = oidc
        .store
        .subject(&session.user.discord_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(ApiError::unauthorized)?;
    let grant =
        oidc.store.grant(&grant_id(&subject, request.client_id)).await.map_err(storage_error)?;

    Ok(BrowserBinding {
        session_id: session.session.id,
        subject,
        grant_version: grant.map(|grant| grant.version),
    })
}

async fn consume(
    oidc: &Oidc,
    request: &Request,
    session: &AuthenticatedSession,
) -> Result<(), ApiError> {
    if !oidc
        .store
        .consume_request(&request.id, &request.browser_hash, session.session.id, DateTime::now())
        .await
        .map_err(storage_error)?
    {
        return Err(invalid_request());
    }
    Ok(())
}

async fn bound_request(
    oidc: &Oidc,
    id: &str,
    headers: &HeaderMap,
    session: &AuthenticatedSession,
    now: DateTime,
) -> Result<Request, ApiError> {
    let request = load_request(oidc, id, &browser_hash(headers)?, now).await?;
    if !request.binding.as_ref().is_some_and(|binding| binding.session_id == session.session.id) {
        return Err(invalid_request());
    }
    Ok(request)
}

async fn load_request(
    oidc: &Oidc,
    id: &str,
    browser_hash: &str,
    now: DateTime,
) -> Result<Request, ApiError> {
    if !opaque_token(id) {
        return Err(invalid_request());
    }
    oidc.store
        .request(id, browser_hash, now)
        .await
        .map_err(storage_error)?
        .ok_or_else(invalid_request)
}

pub(crate) async fn browser_continuation(
    state: &AppState,
    headers: &HeaderMap,
    id: &str,
    denied: bool,
) -> Result<Option<Response>, ApiError> {
    let oidc = provider(state)?;
    let browser = browser_hash(headers)?;
    let now = DateTime::now();
    let request = load_request(oidc, id, &browser, now).await?;
    if request.binding.is_some() {
        return Err(invalid_request());
    }
    if !denied {
        return Ok(None);
    }

    let app = oidc
        .eligible_app(state.auth_store.as_ref(), request.client_id)
        .await?
        .ok_or_else(invalid_callback)?;
    let redirect = registered_redirect(&app, &request.redirect_uri).ok_or_else(invalid_callback)?;
    if !oidc.store.cancel_unbound_request(id, &browser, now).await.map_err(storage_error)? {
        return Err(invalid_request());
    }

    let mut response = callback(redirect, Some(&request.state), "error", "access_denied");
    super::set_private_headers(response.headers_mut());
    Ok(Some(response))
}

pub(crate) fn resume_path(id: &str) -> String {
    format!("/proxy/v1/auth/authorize/resume?request={id}")
}

fn browser_value(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|header| extract_cookie_value(Some(header), BROWSER_COOKIE))
        .filter(|value| opaque_token(value))
}

fn browser_hash(headers: &HeaderMap) -> Result<String, ApiError> {
    browser_value(headers).as_deref().map(hash).ok_or_else(invalid_request)
}

fn with_browser_cookie(mut response: Response, browser: &str) -> Result<Response, ApiError> {
    let cookie = HeaderValue::from_str(&format!(
        "{BROWSER_COOKIE}={browser}; Max-Age=300; Path=/; HttpOnly; Secure; SameSite=Lax"
    ))
    .map_err(|_error| ApiError::internal("invalid authorization cookie"))?;
    response.headers_mut().append(header::SET_COOKIE, cookie);
    Ok(response)
}

impl AuthorizeQuery {
    fn valid_state(&self) -> Option<&str> {
        self.state.as_deref().filter(|value| bounded_text(value))
    }
}

fn bounded_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
}

fn validate_query(query: &AuthorizeQuery) -> Result<Vec<Scope>, &'static str> {
    if query.response_type.as_deref() != Some("code") {
        return Err("unsupported_response_type");
    }
    if query.request.is_some() {
        return Err("request_not_supported");
    }
    if query.request_uri.is_some() {
        return Err("request_uri_not_supported");
    }
    if !matches!(query.prompt.as_deref(), None | Some("consent" | "none"))
        || !matches!(query.response_mode.as_deref(), None | Some("query"))
        || query.valid_state().is_none()
        || !query.nonce.as_deref().is_some_and(bounded_text)
        || !query.code_challenge.as_deref().is_some_and(opaque_token)
        || query.code_challenge_method.as_deref() != Some("S256")
    {
        return Err("invalid_request");
    }

    let scopes = Scope::parse(query.scope.as_deref().ok_or("invalid_scope")?)?;
    if !scopes.contains(&Scope::OpenId) {
        return Err("invalid_scope");
    }
    Ok(scopes)
}

/// Returns a registered callback that can receive unambiguous code flow responses.
pub(super) fn registered_redirect(app: &DeveloperApp, raw: &str) -> Option<Url> {
    if !app.settings.redirect_uris.iter().any(|uri| uri == raw)
        || !crate::routes::developer_apps::is_redirect_uri(raw)
    {
        return None;
    }

    let url = Url::parse(raw).ok()?;
    if url.query_pairs().any(|(name, _)| {
        matches!(
            name.as_ref(),
            "code" | "state" | "iss" | "error" | "error_description" | "error_uri"
        )
    }) {
        return None;
    }
    Some(url)
}

fn callback(mut redirect: Url, state: Option<&str>, key: &str, value: &str) -> Response {
    redirect.query_pairs_mut().append_pair(key, value);
    if let Some(state) = state {
        redirect.query_pairs_mut().append_pair("state", state);
    }
    // The redirect can follow the user's consent POST. A 303 drops that body.
    Redirect::to(redirect.as_str()).into_response()
}

fn invalid_callback() -> ApiError {
    ApiError::bad_request("invalid client or redirect URI")
}

fn invalid_request() -> ApiError {
    ApiError::bad_request("authorization request expired or invalid")
}

fn stale_consent() -> ApiError {
    ApiError::conflict("app authorization changed; restart authorization")
}

#[cfg(test)]
mod tests {
    use axum::{
        http::{Method, StatusCode, header},
        response::Response,
    };
    use reqwest::Url;
    use serde_json::json;

    use super::{BROWSER_COOKIE, mask_email};
    use crate::oidc::test_support::{CALLBACK, Fixture, ORIGIN, VERIFIER, body, location, query};

    #[test]
    fn masks_the_email_local_part_after_the_first_character() {
        assert_eq!(mask_email("private@example.com"), "p******@example.com");
        assert_eq!(mask_email("åβ@example.com"), "å*@example.com");
        assert_eq!(mask_email("a@example.com"), "a@example.com");
    }

    #[test]
    fn masks_an_email_with_a_missing_part_entirely() {
        for email in ["", "private", "@example.com", "private@"] {
            assert_eq!(mask_email(email), "•••");
        }
    }

    #[tokio::test]
    async fn consent_rejects_cross_origin_and_missing_browser_cookie() {
        let fixture = Fixture::new().await;
        let (review, cookie) = fixture.start("openid profile:read").await;
        let bad_origin = fixture.decide(&review, &cookie, "https://foreign.example", "allow").await;
        assert_eq!(bad_origin.status(), StatusCode::FORBIDDEN);
        let bad_browser = fixture.decide(&review, "sid=alice", ORIGIN, "allow").await;
        assert_eq!(bad_browser.status(), StatusCode::BAD_REQUEST);
        let denied = fixture.decide(&review, &cookie, ORIGIN, "deny").await;
        assert_eq!(
            query(&Url::parse(location(&denied)).expect("callback"), "error"),
            "access_denied"
        );
    }

    #[tokio::test]
    async fn repeat_authorization_shows_the_requested_scopes_and_rejects_silent_consent() {
        let fixture = Fixture::new().await;
        let code = fixture.code("openid profile:read").await;
        assert_eq!(fixture.exchange(&code, VERIFIER).await.status(), StatusCode::OK);

        let (review, cookie) = fixture.start("openid").await;
        let consent = review.replacen("/oauth/consent", "/v1/auth/authorize/consent", 1);
        let response =
            fixture.call(Method::GET, &consent, &[("cookie", &cookie)], String::new()).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body(response).await["permissions"],
            json!([{
                "id": "openid",
                "description": "Sign in with ROK Battles",
            }])
        );

        let silent = format!("{}&prompt=none", fixture.authorization_url("openid", CALLBACK));
        let response =
            fixture.call(Method::GET, &silent, &[("cookie", &cookie)], String::new()).await;
        let callback = Url::parse(location(&response)).expect("client callback");
        assert_eq!(query(&callback, "error"), "consent_required");
        assert!(!callback.query_pairs().any(|(name, _)| name == "code"));
    }

    #[tokio::test]
    async fn consent_email_is_masked_until_revealed_in_the_bound_session() {
        let fixture = Fixture::new().await;
        let (review, cookie) = fixture.start("openid").await;
        let consent = review.replacen("/oauth/consent", "/v1/auth/authorize/consent", 1);
        let response =
            fixture.call(Method::GET, &consent, &[("cookie", &cookie)], String::new()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let consent_json = body(response).await;
        assert_eq!(consent_json["maskedEmail"], "p******@example.com");
        assert!(consent_json.get("email").is_none());
        assert!(consent_json.get("discordId").is_none());

        let reveal = consent.replacen("/consent?", "/consent/email?", 1);
        let browser = cookie.strip_prefix("sid=alice; ").expect("browser");
        for (cookie, status) in [
            (browser, StatusCode::UNAUTHORIZED),
            ("sid=alice", StatusCode::BAD_REQUEST),
            (&format!("sid=bob; {browser}"), StatusCode::BAD_REQUEST),
        ] {
            let response =
                fixture.call(Method::GET, &reveal, &[("cookie", cookie)], String::new()).await;
            assert_eq!(response.status(), status);
        }

        let revealed =
            fixture.call(Method::GET, &reveal, &[("cookie", &cookie)], String::new()).await;
        assert_eq!(revealed.status(), StatusCode::OK);
        assert_eq!(revealed.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(body(revealed).await, json!({ "email": "private@example.com" }));
    }

    #[tokio::test]
    async fn switching_accounts_ends_the_session_and_requires_consent_again() {
        let fixture = Fixture::new().await;
        let (review, cookie) = fixture.start("openid profile:read").await;
        let denied =
            fixture.decide(&review, &cookie, "https://other.example", "switch_account").await;
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);

        let switched = fixture.decide(&review, &cookie, ORIGIN, "switch_account").await;
        assert_eq!(switched.status(), StatusCode::SEE_OTHER);
        let discord = Url::parse(location(&switched)).expect("Discord authorize");
        assert_eq!(discord.host_str(), Some("discord.com"));
        assert_eq!(query(&discord, "prompt"), "consent");
        let mut response_cookies = switched.headers().get_all(header::SET_COOKIE).iter();
        assert!(
            response_cookies
                .next()
                .expect("cleared session")
                .to_str()
                .unwrap()
                .starts_with("sid=; Max-Age=0")
        );
        let browser = response_cookies
            .next()
            .expect("browser cookie")
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .expect("browser");
        assert!(browser.starts_with(BROWSER_COOKIE));

        let old_session = fixture.decide(&review, &cookie, ORIGIN, "allow").await;
        assert_eq!(old_session.status(), StatusCode::UNAUTHORIZED);

        let upstream_state = query(&discord, "state");
        let login = fixture
            .call(
                Method::GET,
                &format!(
                    "/v1/auth/discord/callback?state={upstream_state}&code=fake-upstream-code"
                ),
                &[("cookie", browser)],
                String::new(),
            )
            .await;
        assert_eq!(login.status(), StatusCode::SEE_OTHER);
        let cookies = format!("{}; {browser}", first_cookie(&login));
        let continued = fixture
            .call(Method::GET, location(&login), &[("cookie", &cookies)], String::new())
            .await;
        assert_eq!(continued.status(), StatusCode::SEE_OTHER);
        let new_review = location(&continued);
        assert!(new_review.starts_with("/oauth/consent?request="));
        assert_ne!(new_review, review);

        let approved = fixture.decide(new_review, &cookies, ORIGIN, "allow").await;
        let callback = Url::parse(location(&approved)).expect("client callback");
        assert_eq!(query(&callback, "state"), "client-state");
        let code = query(&callback, "code");
        let exchanged = fixture.exchange(&code, VERIFIER).await;
        assert_eq!(exchanged.status(), StatusCode::OK);
        assert_eq!(body(exchanged).await["scope"], "openid profile:read");
    }

    #[tokio::test]
    async fn validates_redirects_and_binds_discord_continuation_to_the_browser() {
        let fixture = Fixture::new().await;
        let unsafe_callback =
            fixture.authorization_url("openid", "https://foreign.example/callback");
        let rejected = fixture.call(Method::GET, &unsafe_callback, &[], String::new()).await;
        assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
        assert!(!rejected.headers().contains_key(header::LOCATION));

        let unknown_scope = fixture.authorization_url("openid offline_access", CALLBACK);
        let refused = fixture.call(Method::GET, &unknown_scope, &[], String::new()).await;
        assert_eq!(
            query(&Url::parse(location(&refused)).expect("callback"), "error"),
            "invalid_scope"
        );

        let missing_openid = fixture.authorization_url("profile:read", CALLBACK);
        let refused = fixture.call(Method::GET, &missing_openid, &[], String::new()).await;
        assert_eq!(
            query(&Url::parse(location(&refused)).expect("callback"), "error"),
            "invalid_scope"
        );

        let start = fixture.authorization_url("openid", CALLBACK);
        let started = fixture.call(Method::GET, &start, &[], String::new()).await;
        let discord = Url::parse(location(&started)).expect("Discord authorize");
        assert_eq!(discord.host_str(), Some("discord.com"));
        let upstream_state = query(&discord, "state");
        assert_ne!(upstream_state, "client-state");
        assert!(
            started
                .headers()
                .get(header::SET_COOKIE)
                .expect("browser cookie")
                .to_str()
                .expect("cookie")
                .starts_with(BROWSER_COOKIE)
        );

        let response = fixture
            .call(
                Method::GET,
                &format!("/v1/auth/discord/callback?state={upstream_state}&error=access_denied"),
                &[],
                String::new(),
            )
            .await;
        assert_eq!(body(response).await["error"], "authorization request expired or invalid");
    }

    #[tokio::test]
    async fn signed_out_authorization_resumes_after_the_discord_callback() {
        let fixture = Fixture::new().await;
        let start = fixture.authorization_url("openid", CALLBACK);
        let started = fixture.call(Method::GET, &start, &[], String::new()).await;
        let upstream = Url::parse(location(&started)).expect("Discord authorize");
        let upstream_state = query(&upstream, "state");
        let browser = first_cookie(&started);

        let login = fixture
            .call(
                Method::GET,
                &format!(
                    "/v1/auth/discord/callback?state={upstream_state}&code=fake-upstream-code"
                ),
                &[("cookie", browser)],
                String::new(),
            )
            .await;
        assert_eq!(login.status(), StatusCode::SEE_OTHER);
        assert_eq!(login.headers().get(header::CACHE_CONTROL).expect("cache control"), "no-store");
        let cookies = format!("{}; {browser}", first_cookie(&login));
        let continued = fixture
            .call(Method::GET, location(&login), &[("cookie", &cookies)], String::new())
            .await;
        assert_eq!(continued.status(), StatusCode::SEE_OTHER);
        let approved = fixture.decide(location(&continued), &cookies, ORIGIN, "allow").await;
        let callback = Url::parse(location(&approved)).expect("client callback");
        assert_eq!(query(&callback, "state"), "client-state");
        let code = query(&callback, "code");
        assert_eq!(fixture.exchange(&code, VERIFIER).await.status(), StatusCode::OK);
    }

    fn first_cookie(response: &Response) -> &str {
        response
            .headers()
            .get(header::SET_COOKIE)
            .expect("cookie")
            .to_str()
            .expect("cookie string")
            .split(';')
            .next()
            .expect("cookie value")
    }
}
