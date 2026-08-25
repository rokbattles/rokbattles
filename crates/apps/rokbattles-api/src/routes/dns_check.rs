use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{AUTHORIZATION, CACHE_CONTROL},
    },
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::{error::ApiError, state::AppState};

#[derive(Deserialize)]
struct MarkRequest {
    nonce: String,
}

#[derive(Serialize)]
struct CheckResponse {
    active: bool,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/mark", post(mark)).route("/{nonce}", get(check))
}

async fn mark(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<MarkRequest>,
) -> Result<StatusCode, ApiError> {
    if !is_authorized(&headers, &state.dns_check_secret) {
        return Err(ApiError::unauthorized());
    }
    validate_nonce(&request.nonce)?;
    state
        .dns_check_store
        .mark(&request.nonce)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn check(
    State(state): State<Arc<AppState>>,
    Path(nonce): Path<String>,
) -> Result<Response, ApiError> {
    validate_nonce(&nonce)?;
    let active = state
        .dns_check_store
        .is_active(&nonce)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let mut response = Json(CheckResponse { active }).into_response();
    response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

fn validate_nonce(nonce: &str) -> Result<(), ApiError> {
    if nonce.len() == 32
        && nonce.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Ok(());
    }
    Err(ApiError::bad_request("invalid DNS check nonce"))
}

fn is_authorized(headers: &HeaderMap, secret: &str) -> bool {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|value| value == secret)
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn nonce_should_be_exactly_128_bits_of_lowercase_hex() {
        assert!(validate_nonce("0123456789abcdef0123456789abcdef").is_ok());
        for invalid in
            ["short", "0123456789abcdef0123456789abcdeg", "0123456789ABCDEF0123456789ABCDEF"]
        {
            assert!(validate_nonce(invalid).is_err());
        }
    }

    #[test]
    fn callback_should_require_exact_bearer_secret() {
        let mut headers = HeaderMap::new();
        assert!(!is_authorized(&headers, "shared-secret"));
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer wrong-secret"));
        assert!(!is_authorized(&headers, "shared-secret"));
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer shared-secret"));
        assert!(is_authorized(&headers, "shared-secret"));
    }
}
