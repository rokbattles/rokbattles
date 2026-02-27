use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{Json, http::StatusCode};

use crate::error::ApiError;
use crate::state::AppState;

use self::auth::is_authorized_request;
use self::store::refresh_claimed_governor_bindings;
use self::types::RefreshBindsResponse;

mod auth;
mod store;
mod types;

/// Refresh claimed governor names and avatars from the latest battle data.
pub async fn post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    refresh_binds(state, headers).await
}

async fn refresh_binds(
    state: Arc<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    if !is_authorized_request(&headers, &state.cron_secret) {
        return Err(ApiError::unauthorized());
    }

    let stats = refresh_claimed_governor_bindings(
        state.reports_store.claimed_governors_collection(),
        state.reports_store.battle_collection(),
    )
    .await?;

    let response = RefreshBindsResponse::from(stats);
    Ok((
        StatusCode::OK,
        [("Cache-Control", "no-store")],
        Json(response),
    ))
}
