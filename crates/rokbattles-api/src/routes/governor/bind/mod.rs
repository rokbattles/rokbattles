use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::{Json, http::StatusCode};

use crate::auth::AuthenticatedSession;
use crate::error::ApiError;
use crate::routes::governor::common::parse_governor_id_param;
use crate::routes::governor::snapshot::find_latest_sender_snapshot;
use crate::state::AppState;

use self::store::{
    claim_document_is_default, claim_exists_for_user, delete_claim_for_user,
    find_most_recent_governor_id, governor_claim_exists, insert_claim, set_bind_as_default,
    summarize_user_claims,
};
use self::types::{BindGovernorResponse, ClaimedGovernor};

mod store;
mod types;

const MAX_GOVERNOR_BINDS: u64 = 3;

/// Claim a governor ID for the authenticated user.
pub async fn post(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let claims = state.reports_store.claimed_governors_collection();

    if governor_claim_exists(claims, governor_id).await? {
        return Err(ApiError::conflict("Governor already claimed"));
    }

    let summary =
        summarize_user_claims(claims, &session.user.discord_id, MAX_GOVERNOR_BINDS).await?;
    if summary.claim_count >= MAX_GOVERNOR_BINDS {
        return Err(ApiError::conflict("Claim limit reached"));
    }

    let default = !summary.has_default_claim;
    let (governor_name, governor_avatar) =
        find_latest_sender_snapshot(state.reports_store.battle_collection(), governor_id)
            .await?
            .map(|snapshot| (snapshot.governor_name, snapshot.governor_avatar))
            .unwrap_or_default();

    insert_claim(
        claims,
        &session.user.discord_id,
        governor_id,
        governor_name.as_deref(),
        governor_avatar.as_deref(),
        default,
    )
    .await?;

    Ok((
        StatusCode::OK,
        [("Cache-Control", "no-store")],
        Json(BindGovernorResponse {
            claim: ClaimedGovernor {
                governor_id,
                governor_name,
                governor_avatar,
                default,
            },
        }),
    ))
}

/// Set one of the authenticated user's binds as default.
pub async fn patch_default(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let claims = state.reports_store.claimed_governors_collection();

    if !claim_exists_for_user(claims, &session.user.discord_id, governor_id).await? {
        return Err(ApiError::not_found("Claim not found"));
    }

    set_bind_as_default(claims, &session.user.discord_id, governor_id).await?;
    Ok((StatusCode::NO_CONTENT, [("Cache-Control", "no-store")]))
}

/// Delete one claimed governor bind from the authenticated user.
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let claims = state.reports_store.claimed_governors_collection();
    let deleted_claim =
        delete_claim_for_user(claims, &session.user.discord_id, governor_id).await?;

    let Some(deleted_claim) = deleted_claim else {
        return Err(ApiError::not_found("Claim not found"));
    };

    if claim_document_is_default(&deleted_claim)
        && let Some(next_default_governor_id) =
            find_most_recent_governor_id(claims, &session.user.discord_id).await?
    {
        set_bind_as_default(claims, &session.user.discord_id, next_default_governor_id).await?;
    }

    Ok((StatusCode::NO_CONTENT, [("Cache-Control", "no-store")]))
}
