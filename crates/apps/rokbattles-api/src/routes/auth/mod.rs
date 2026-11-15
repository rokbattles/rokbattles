use std::{sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{
        HeaderMap,
        header::{COOKIE, SET_COOKIE},
    },
    middleware,
    response::{IntoResponse, Redirect, Response},
    routing::{MethodRouter, get, post},
};
use futures::StreamExt;
use mongodb::{
    bson::{Bson, DateTime, Document, doc},
    options::FindOptions,
};
use reqwest::Url;
use rokbattles_bson::bson_to_i64_exact;
use serde::{Deserialize, de::DeserializeOwned};

use crate::{
    auth::{AuthenticatedSession, extract_cookie_value},
    db::{DesktopLogin, DiscordUserUpsert, NewSessionRecord, OAuthStateRecord},
    error::ApiError,
    rate_limit::{self, RateLimitMode, RateLimiter},
    state::{AppState, DiscordOAuthConfig},
};

mod cookies;
mod desktop;
mod oauth;
mod types;

use self::types::{AuthMeResponse, ClaimedGovernor, CurrentUser, LogoutResponse};

const OAUTH_STATE_TTL_MILLIS: i64 = 10 * 60 * 1000;
const SESSION_TTL_MILLIS: i64 = 7 * 24 * 60 * 60 * 1000;
const SESSION_MAX_AGE_SECONDS: u64 = 7 * 24 * 60 * 60;

const LOGIN_BURST: u32 = 10;
const LOGIN_REFILL: Duration = Duration::from_secs(30);
const EXCHANGE_BURST: u32 = 15;
const EXCHANGE_REFILL: Duration = Duration::from_secs(10);

pub fn router(rate_limit_mode: RateLimitMode) -> Router<Arc<AppState>> {
    Router::new()
        .route("/me", get(get_me))
        .route("/logout", post(post_logout))
        .route("/discord/callback", get(get_discord_callback))
        .merge(limited_routes(
            rate_limit_mode,
            get(get_discord_login),
            get(desktop::login),
            post(desktop::exchange),
        ))
}

fn limited_routes<S: Clone + Send + Sync + 'static>(
    mode: RateLimitMode,
    website_login: MethodRouter<S>,
    desktop_login: MethodRouter<S>,
    desktop_exchange: MethodRouter<S>,
) -> Router<S> {
    let login = Arc::new(RateLimiter::new(mode, LOGIN_BURST, LOGIN_REFILL));
    let exchange = Arc::new(RateLimiter::new(mode, EXCHANGE_BURST, EXCHANGE_REFILL));
    Router::new()
        .route("/discord/login", website_login)
        .route("/desktop/login", desktop_login)
        .route_layer(middleware::from_fn_with_state(login, rate_limit::enforce))
        .merge(
            Router::new()
                .route("/desktop/exchange", desktop_exchange)
                .route_layer(middleware::from_fn_with_state(exchange, rate_limit::enforce)),
        )
}

async fn get_me(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let claimed_governors = load_claimed_governors(&state, &session.user.discord_id).await?;

    Ok((
        [("Cache-Control", "no-store")],
        Json(AuthMeResponse {
            user: CurrentUser {
                discord_id: session.user.discord_id,
                email: session.user.email,
                roles: session.user.roles,
                claimed_governors,
            },
        }),
    ))
}

async fn post_logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    if let Some(sid) =
        extract_cookie_value(headers.get(COOKIE).and_then(|header| header.to_str().ok()), "sid")
    {
        state
            .auth_store
            .delete_session_by_id(&sid)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
    }

    let mut response = Json(LogoutResponse { success: true }).into_response();
    let clear_cookie = cookies::build_clear_session_cookie();
    response.headers_mut().append(
        SET_COOKIE,
        clear_cookie
            .parse()
            .map_err(|error| ApiError::internal(format!("invalid cookie header: {error}")))?,
    );

    Ok(response)
}

async fn get_discord_login(State(state): State<Arc<AppState>>) -> Result<Redirect, ApiError> {
    begin_discord_login(&state, None).await
}

async fn begin_discord_login(
    state: &AppState,
    desktop: Option<DesktopLogin>,
) -> Result<Redirect, ApiError> {
    let verifier = oauth::generate_code_verifier();
    let challenge = oauth::derive_code_challenge(&verifier);
    let state_token = oauth::generate_state();
    let now = DateTime::now();
    let expires_at = DateTime::from_millis(now.timestamp_millis() + OAUTH_STATE_TTL_MILLIS);

    state
        .auth_store
        .insert_oauth_state(OAuthStateRecord {
            state: state_token.clone(),
            verifier,
            created_at: now,
            expires_at,
            desktop,
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut redirect_url = Url::parse("https://discord.com/oauth2/authorize")
        .map_err(|error| ApiError::internal(format!("invalid discord authorize url: {error}")))?;
    redirect_url
        .query_pairs_mut()
        .append_pair("client_id", &state.discord_oauth.client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &state.discord_oauth.redirect_uri)
        .append_pair("scope", "identify email")
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &state_token);

    Ok(Redirect::temporary(redirect_url.as_ref()))
}

async fn get_discord_callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DiscordCallbackQuery>,
) -> Result<Response, ApiError> {
    let Some(state_token) = query.state else {
        return Err(ApiError::bad_request("missing state"));
    };

    let Some(oauth_state) = state
        .auth_store
        .consume_oauth_state(&state_token)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
    else {
        return Err(ApiError::bad_request("invalid or expired state"));
    };

    if oauth_state.expires_at.timestamp_millis() <= DateTime::now().timestamp_millis() {
        return Err(ApiError::bad_request("invalid or expired state"));
    }

    if query.error.is_some() {
        return match oauth_state.desktop {
            Some(login) => desktop::callback(&login, "error", "access_denied"),
            None => Err(ApiError::bad_request("discord authorization denied")),
        };
    }

    let Some(code) = query.code else {
        return Err(ApiError::bad_request("missing code"));
    };

    let desktop_login = oauth_state.desktop.clone();
    match (finish_discord_login(&state, &code, oauth_state).await, desktop_login) {
        (Err(error), Some(login)) => {
            tracing::warn!(%error, "desktop Discord sign-in failed");
            desktop::callback(&login, "error", "sign_in_failed")
        }
        (result, _) => result,
    }
}

async fn finish_discord_login(
    state: &AppState,
    code: &str,
    oauth_state: OAuthStateRecord,
) -> Result<Response, ApiError> {
    let token = exchange_discord_token(&state.discord_oauth, code, &oauth_state.verifier).await?;
    let profile = fetch_discord_profile(&token.access_token).await?;

    let Some(email) = profile.email else {
        return Err(ApiError::bad_request("discord email not verified"));
    };
    if !profile.verified.unwrap_or(false) {
        return Err(ApiError::bad_request("discord email not verified"));
    }

    state
        .auth_store
        .upsert_discord_user(DiscordUserUpsert { discord_id: profile.id.clone(), email })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    if let Some(login) = oauth_state.desktop {
        return desktop::authorize(state, &login, profile.id).await;
    }

    let now = DateTime::now();
    let expires_at = DateTime::from_millis(now.timestamp_millis() + SESSION_TTL_MILLIS);
    let sid = oauth::generate_session_id();
    state
        .auth_store
        .insert_session(NewSessionRecord {
            session_id: sid.clone(),
            user_id: profile.id,
            created_at: now,
            expires_at,
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut response = Redirect::to("/").into_response();
    let set_cookie = cookies::build_set_session_cookie(&sid, SESSION_MAX_AGE_SECONDS);
    response.headers_mut().append(
        SET_COOKIE,
        set_cookie
            .parse()
            .map_err(|error| ApiError::internal(format!("invalid cookie header: {error}")))?,
    );

    Ok(response)
}

async fn load_claimed_governors(
    state: &Arc<AppState>,
    discord_id: &str,
) -> Result<Vec<ClaimedGovernor>, ApiError> {
    let options = FindOptions::builder()
        .sort(doc! { "default": -1, "createdAt": -1 })
        .projection(doc! {
            "_id": 0,
            "governorId": 1,
            "governorName": 1,
            "governorAvatar": 1,
            "createdAt": 1,
            "default": 1
        })
        .build();

    let claims_collection = state.reports_store.claimed_governors_collection();
    let mut cursor = claims_collection
        .find(doc! { "discordId": discord_id })
        .with_options(options)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut claims = Vec::new();
    while let Some(next) = cursor.next().await {
        let document = next.map_err(|error| ApiError::internal(error.to_string()))?;
        let Some(governor_id) = document.get("governorId").and_then(bson_to_i64_exact) else {
            continue;
        };

        claims.push(ClaimedGovernor {
            governor_id,
            governor_name: optional_string_field(&document, "governorName"),
            governor_avatar: optional_string_field(&document, "governorAvatar"),
            default: document.get_bool("default").unwrap_or(false),
        });
    }

    Ok(claims)
}

fn optional_string_field(document: &Document, key: &str) -> Option<String> {
    document.get(key).and_then(Bson::as_str).map(ToString::to_string)
}

async fn exchange_discord_token(
    oauth: &DiscordOAuthConfig,
    code: &str,
    verifier: &str,
) -> Result<DiscordTokenResponse, ApiError> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://discord.com/api/oauth2/token")
        .basic_auth(&oauth.client_id, Some(&oauth.client_secret))
        .form(&[
            ("client_id", oauth.client_id.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", oauth.redirect_uri.as_str()),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .map_err(|error| discord_request_error("token_exchange", error))?;

    decode_discord_response(response, "token_exchange").await
}

async fn fetch_discord_profile(access_token: &str) -> Result<DiscordProfileResponse, ApiError> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://discord.com/api/users/@me")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|error| discord_request_error("profile", error))?;

    decode_discord_response(response, "profile").await
}

async fn decode_discord_response<T: DeserializeOwned>(
    response: reqwest::Response,
    operation: &'static str,
) -> Result<T, ApiError> {
    if !response.status().is_success() {
        tracing::warn!(
            operation,
            status = response.status().as_u16(),
            "Discord OAuth request rejected"
        );
        return Err(ApiError::bad_request("discord sign-in failed"));
    }

    response.json().await.map_err(|error| discord_request_error(operation, error))
}

fn discord_request_error(operation: &'static str, error: reqwest::Error) -> ApiError {
    tracing::warn!(
        operation,
        timeout = error.is_timeout(),
        connection = error.is_connect(),
        decode = error.is_decode(),
        "Discord OAuth request failed"
    );
    ApiError::internal("discord sign-in failed")
}

#[derive(Debug, Deserialize)]
struct DiscordCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DiscordTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct DiscordProfileResponse {
    id: String,
    email: Option<String>,
    verified: Option<bool>,
}

#[cfg(test)]
mod tests {
    use mongodb::bson::doc;

    use super::*;

    fn limiter_router(mode: RateLimitMode) -> Router {
        limited_routes(
            mode,
            get(|| async { axum::http::StatusCode::NO_CONTENT }),
            get(|| async { axum::http::StatusCode::NO_CONTENT }),
            post(|| async { axum::http::StatusCode::NO_CONTENT }),
        )
    }

    fn limiter_request(
        method: axum::http::Method,
        path: &str,
    ) -> axum::http::Request<axum::body::Body> {
        axum::http::Request::builder()
            .method(method)
            .uri(path)
            .header("cf-connecting-ip", "192.0.2.1")
            .body(axum::body::Body::empty())
            .expect("request")
    }

    #[tokio::test]
    async fn website_and_desktop_login_share_a_budget_but_exchange_is_independent() {
        use axum::http::{Method, StatusCode, header::RETRY_AFTER};
        use tower::ServiceExt;

        let app = limiter_router(RateLimitMode::Cloudflare);

        for attempt in 0..LOGIN_BURST {
            let path = if attempt % 2 == 0 { "/discord/login" } else { "/desktop/login" };
            let response =
                app.clone().oneshot(limiter_request(Method::GET, path)).await.expect("response");
            assert_eq!(response.status(), StatusCode::NO_CONTENT);
        }

        for path in ["/discord/login", "/desktop/login"] {
            let response =
                app.clone().oneshot(limiter_request(Method::GET, path)).await.expect("response");
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
            let seconds: u64 = response.headers()[RETRY_AFTER]
                .to_str()
                .expect("retry header")
                .parse()
                .expect("retry seconds");
            assert!((1..=LOGIN_REFILL.as_secs()).contains(&seconds));
        }

        for _ in 0..EXCHANGE_BURST {
            let response = app
                .clone()
                .oneshot(limiter_request(Method::POST, "/desktop/exchange"))
                .await
                .expect("response");
            assert_eq!(response.status(), StatusCode::NO_CONTENT);
        }

        let response = app
            .oneshot(limiter_request(Method::POST, "/desktop/exchange"))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().contains_key(RETRY_AFTER));
    }

    #[tokio::test]
    async fn exchange_exhaustion_does_not_block_login() {
        use axum::http::{Method, StatusCode};
        use tower::ServiceExt;

        let app = limiter_router(RateLimitMode::Cloudflare);

        for _ in 0..EXCHANGE_BURST {
            let response = app
                .clone()
                .oneshot(limiter_request(Method::POST, "/desktop/exchange"))
                .await
                .expect("response");
            assert_eq!(response.status(), StatusCode::NO_CONTENT);
        }

        let response = app
            .clone()
            .oneshot(limiter_request(Method::POST, "/desktop/exchange"))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        let response =
            app.oneshot(limiter_request(Method::GET, "/discord/login")).await.expect("response");
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn malformed_login_queries_and_exchange_bodies_still_spend_capacity() {
        use axum::{
            body::Body,
            http::{Method, Request, StatusCode, header::CONTENT_TYPE},
        };
        use tower::ServiceExt;

        let app = limited_routes(
            RateLimitMode::Cloudflare,
            get(|| async { StatusCode::NO_CONTENT }),
            get(|Query(_login): Query<DesktopLogin>| async { StatusCode::NO_CONTENT }),
            post(|Json(_request): Json<serde_json::Value>| async { StatusCode::NO_CONTENT }),
        );

        for _ in 0..LOGIN_BURST {
            let response = app
                .clone()
                .oneshot(limiter_request(Method::GET, "/desktop/login?port=invalid"))
                .await
                .expect("response");
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }

        let response = app
            .clone()
            .oneshot(limiter_request(Method::GET, "/discord/login"))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        for _ in 0..EXCHANGE_BURST {
            let request = Request::builder()
                .method(Method::POST)
                .uri("/desktop/exchange")
                .header("cf-connecting-ip", "192.0.2.1")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from("{invalid-json"))
                .expect("request");
            let response = app.clone().oneshot(request).await.expect("response");
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }

        let response = app
            .oneshot(limiter_request(Method::POST, "/desktop/exchange"))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn local_mode_needs_no_cloudflare_header_and_never_throttles_auth_routes() {
        use axum::{
            body::Body,
            http::{Method, Request, StatusCode},
        };
        use tower::ServiceExt;

        let app = limiter_router(RateLimitMode::Local);

        for (method, path) in [
            (Method::GET, "/discord/login"),
            (Method::GET, "/desktop/login"),
            (Method::POST, "/desktop/exchange"),
        ] {
            for _ in 0..100 {
                let request = Request::builder()
                    .method(method.clone())
                    .uri(path)
                    .body(Body::empty())
                    .expect("request");
                let response = app.clone().oneshot(request).await.expect("response");
                assert_eq!(response.status(), StatusCode::NO_CONTENT);
            }
        }
    }

    #[tokio::test]
    async fn callback_me_and_logout_remain_outside_the_limiter() {
        use axum::{
            body::Body,
            http::{Method, Request, StatusCode},
        };
        use tower::ServiceExt;

        let app = limiter_router(RateLimitMode::Cloudflare)
            .route("/discord/callback", get(|| async { StatusCode::NO_CONTENT }))
            .route("/me", get(|| async { StatusCode::NO_CONTENT }))
            .route("/logout", post(|| async { StatusCode::NO_CONTENT }));

        for (method, path) in
            [(Method::GET, "/discord/callback"), (Method::GET, "/me"), (Method::POST, "/logout")]
        {
            for _ in 0..100 {
                let request = Request::builder()
                    .method(method.clone())
                    .uri(path)
                    .body(Body::empty())
                    .expect("request");
                let response = app.clone().oneshot(request).await.expect("response");
                assert_eq!(response.status(), StatusCode::NO_CONTENT);
            }
        }
    }

    fn discord_response(status: axum::http::StatusCode, body: &str) -> reqwest::Response {
        axum::http::Response::builder()
            .status(status)
            .body(body.to_string())
            .expect("mock Discord response")
            .into()
    }

    #[tokio::test]
    async fn discord_token_rejections_never_reflect_upstream_bodies() {
        use axum::{body::to_bytes, http::StatusCode};

        for status in [
            StatusCode::BAD_REQUEST,
            StatusCode::UNAUTHORIZED,
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::INTERNAL_SERVER_ERROR,
        ] {
            let upstream = discord_response(
                status,
                r#"{"error":"invalid_grant","error_description":"private-upstream-detail"}"#,
            );
            let error = decode_discord_response::<DiscordTokenResponse>(upstream, "token_exchange")
                .await
                .expect_err("Discord rejected token exchange");
            assert_eq!(error.to_string(), "discord sign-in failed");
            let response = error.into_response();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            let body = to_bytes(response.into_body(), 1024).await.expect("error body");
            let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON error");
            assert_eq!(json, serde_json::json!({"error": "discord sign-in failed"}));
        }
    }

    #[tokio::test]
    async fn discord_profile_rejections_never_reflect_upstream_bodies() {
        use axum::{body::to_bytes, http::StatusCode};

        let upstream = discord_response(StatusCode::FORBIDDEN, "private-profile-details");
        let error = decode_discord_response::<DiscordProfileResponse>(upstream, "profile")
            .await
            .expect_err("Discord rejected profile request");
        assert_eq!(error.to_string(), "discord sign-in failed");
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), 1024).await.expect("error body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON error");
        assert_eq!(json, serde_json::json!({"error": "discord sign-in failed"}));
    }

    #[tokio::test]
    async fn malformed_discord_token_response_is_sanitized() {
        use axum::{body::to_bytes, http::StatusCode};

        let upstream = discord_response(
            StatusCode::OK,
            r#"{"access_token":{"private-upstream-detail":true}}"#,
        );
        let error = decode_discord_response::<DiscordTokenResponse>(upstream, "token_exchange")
            .await
            .expect_err("invalid token payload");
        assert_eq!(error.to_string(), "internal error: discord sign-in failed");
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = to_bytes(response.into_body(), 1024).await.expect("error body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON error");
        assert_eq!(json, serde_json::json!({"error": "internal-server-error"}));
    }

    #[tokio::test]
    async fn malformed_discord_profile_response_is_sanitized() {
        let upstream = discord_response(axum::http::StatusCode::OK, "private-profile-details");
        let error = decode_discord_response::<DiscordProfileResponse>(upstream, "profile")
            .await
            .expect_err("invalid profile payload");
        assert_eq!(error.to_string(), "internal error: discord sign-in failed");
    }

    #[test]
    fn discord_transport_errors_do_not_expose_request_details() {
        let request_error = reqwest::Client::new()
            .get("http://[private-request-detail")
            .build()
            .expect_err("invalid request URL");
        let error = discord_request_error("token_exchange", request_error);
        assert_eq!(error.to_string(), "internal error: discord sign-in failed");
    }

    #[tokio::test]
    async fn successful_discord_responses_are_preserved() {
        let token = decode_discord_response::<DiscordTokenResponse>(
            discord_response(axum::http::StatusCode::OK, r#"{"access_token":"test-access-token"}"#),
            "token_exchange",
        )
        .await
        .expect("token response");
        assert_eq!(token.access_token, "test-access-token");

        let profile = decode_discord_response::<DiscordProfileResponse>(
            discord_response(
                axum::http::StatusCode::OK,
                r#"{"id":"123","email":"test@example.com","verified":true}"#,
            ),
            "profile",
        )
        .await
        .expect("profile response");
        assert_eq!(profile.id, "123");
        assert_eq!(profile.email.as_deref(), Some("test@example.com"));
        assert_eq!(profile.verified, Some(true));
    }

    #[test]
    fn extracts_optional_string_fields() {
        let document = doc! {
            "governorName": "test",
            "governorAvatar": mongodb::bson::Bson::Null
        };

        assert_eq!(optional_string_field(&document, "governorName"), Some("test".to_string()));
        assert_eq!(optional_string_field(&document, "governorAvatar"), None);
        assert_eq!(optional_string_field(&document, "missing"), None);
    }
}
