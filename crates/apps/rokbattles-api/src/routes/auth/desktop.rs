//! Browser OAuth handoff to a loopback listener owned by the desktop app.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use mongodb::bson::DateTime;
use serde::Deserialize;

use super::{cookies, oauth};
use crate::{
    db::{DesktopLogin, DesktopOAuthState, NewSessionRecord},
    error::ApiError,
    state::AppState,
};

const OAUTH_STATE_TTL_MILLIS: i64 = 60_000;
const SESSION_MAX_AGE_SECONDS: u64 = 30 * 24 * 60 * 60;
const SESSION_TTL_MILLIS: i64 = SESSION_MAX_AGE_SECONDS as i64 * 1000;

fn is_token(value: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&value.len())
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn validate_login(login: &DesktopLogin) -> Result<(), ApiError> {
    if login.port < 1024
        || !is_token(&login.state, 32, 128)
        || !is_token(&login.code_challenge, 43, 43)
    {
        return Err(ApiError::bad_request("invalid desktop login request"));
    }

    Ok(())
}

pub(super) async fn login(
    State(state): State<Arc<AppState>>,
    Query(login): Query<DesktopLogin>,
) -> Result<Response, ApiError> {
    validate_login(&login)?;

    Ok(private(super::begin_discord_login(&state, Some(login)).await?.into_response()))
}

pub(super) fn callback(login: &DesktopLogin, key: &str, value: &str) -> Result<Response, ApiError> {
    // Never accept an arbitrary redirect URI from the request or database.
    validate_login(login)?;

    let mut url = reqwest::Url::parse(&format!("http://127.0.0.1:{}/auth/callback", login.port))
        .map_err(|error| ApiError::internal(error.to_string()))?;
    url.query_pairs_mut().append_pair("state", &login.state).append_pair(key, value);

    Ok(private(Redirect::to(url.as_str()).into_response()))
}

pub(super) async fn authorize(
    state: &AppState,
    login: &DesktopLogin,
    user_id: String,
) -> Result<Response, ApiError> {
    let code = oauth::generate_code_verifier();
    state
        .auth_store
        .insert_desktop_oauth_state(DesktopOAuthState {
            code_hash: oauth::derive_code_challenge(&code),
            code_challenge: login.code_challenge.clone(),
            user_id,
            expires_at: DateTime::from_millis(
                DateTime::now().timestamp_millis() + OAUTH_STATE_TTL_MILLIS,
            ),
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    callback(login, "code", &code)
}

#[derive(Deserialize)]
pub(super) struct ExchangeRequest {
    code: String,
    verifier: String,
}

pub(super) async fn exchange(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ExchangeRequest>,
) -> Result<Response, ApiError> {
    if !is_token(&request.code, 43, 128) || !is_token(&request.verifier, 43, 128) {
        return Err(ApiError::bad_request("invalid desktop authorization code"));
    }

    let oauth_state = state
        .auth_store
        .consume_desktop_oauth_state(
            &oauth::derive_code_challenge(&request.code),
            &oauth::derive_code_challenge(&request.verifier),
            DateTime::now(),
        )
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(ApiError::unauthorized)?;

    let now = DateTime::now();
    let sid = oauth::generate_session_id();
    state
        .auth_store
        .insert_session(NewSessionRecord {
            session_id: sid.clone(),
            user_id: oauth_state.user_id,
            created_at: now,
            expires_at: DateTime::from_millis(now.timestamp_millis() + SESSION_TTL_MILLIS),
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut response = private(StatusCode::NO_CONTENT.into_response());
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookies::build_set_session_cookie(&sid, SESSION_MAX_AGE_SECONDS)
            .parse()
            .map_err(|error| ApiError::internal(format!("invalid session cookie: {error}")))?,
    );

    Ok(response)
}

fn private(mut response: Response) -> Response {
    response.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn login() -> DesktopLogin {
        DesktopLogin { port: 54321, state: "a".repeat(43), code_challenge: "b".repeat(43) }
    }

    #[test]
    fn callback_is_fixed_to_loopback_and_does_not_cache_the_code() {
        let response = callback(&login(), "code", "test_code").expect("callback");
        let url: reqwest::Url =
            response.headers()[header::LOCATION].to_str().expect("header").parse().expect("url");

        assert_eq!(url.origin().ascii_serialization(), "http://127.0.0.1:54321");
        assert_eq!(url.path(), "/auth/callback");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(response.headers()[header::REFERRER_POLICY], "no-referrer");
    }

    #[test]
    fn rejects_privileged_ports_and_invalid_pkce_or_state() {
        let mut request = login();
        validate_login(&request).expect("valid request");

        request.port = 80;
        validate_login(&request).expect_err("invalid request");

        request = login();
        request.state = "too-short".into();
        validate_login(&request).expect_err("invalid request");

        request = login();
        request.code_challenge = "https://example.com/".into();
        validate_login(&request).expect_err("invalid request");
    }
}
