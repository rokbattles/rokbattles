use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use self::{
    store::{
        claim_document_is_default, claim_exists_for_user, delete_claim_for_user,
        find_default_or_latest_governor_id, governor_claim_exists, insert_claim,
        set_bind_as_default, summarize_user_claims,
    },
    types::{BindGovernorResponse, ClaimedGovernor},
};
use crate::{
    error::ApiError,
    routes::{
        governor::{common::parse_governor_id_param, snapshot::find_latest_sender_snapshot},
        me::AccountAccess,
    },
    state::AppState,
};

mod store;
mod types;

const MAX_GOVERNOR_BINDS: u64 = 10;

fn can_claim_more_governors(claim_count: u64) -> bool {
    claim_count < MAX_GOVERNOR_BINDS
}

/// Claim a governor ID for the authenticated user.
pub async fn post(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    access: AccountAccess,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let _operation = state.account_operations.lock(access.account_id()).await;
    access.ensure_active(&state).await?;

    let claims = state.reports_store.claimed_governors_collection();

    if governor_claim_exists(claims, governor_id).await? {
        return Err(ApiError::conflict("Governor already claimed"));
    }

    let summary = summarize_user_claims(claims, access.account_id(), MAX_GOVERNOR_BINDS).await?;
    if !can_claim_more_governors(summary.claim_count) {
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
        access.account_id(),
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
            claim: ClaimedGovernor { governor_id, governor_name, governor_avatar, default },
        }),
    ))
}

/// Set one of the authenticated user's binds as default.
pub async fn patch_default(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    access: AccountAccess,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let _operation = state.account_operations.lock(access.account_id()).await;
    access.ensure_active(&state).await?;

    let claims = state.reports_store.claimed_governors_collection();

    if !claim_exists_for_user(claims, access.account_id(), governor_id).await? {
        return Err(ApiError::not_found("Claim not found"));
    }

    set_bind_as_default(claims, access.account_id(), governor_id).await?;
    Ok((StatusCode::NO_CONTENT, [("Cache-Control", "no-store")]))
}

/// Delete one claimed governor bind from the authenticated user.
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    access: AccountAccess,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let _operation = state.account_operations.lock(access.account_id()).await;
    access.ensure_active(&state).await?;

    let claims = state.reports_store.claimed_governors_collection();
    let deleted_claim = delete_claim_for_user(claims, access.account_id(), governor_id).await?;

    let Some(deleted_claim) = deleted_claim else {
        return Err(ApiError::not_found("Claim not found"));
    };

    if claim_document_is_default(&deleted_claim)
        && let Some(next_default_governor_id) =
            find_default_or_latest_governor_id(claims, access.account_id()).await?
    {
        set_bind_as_default(claims, access.account_id(), next_default_governor_id).await?;
    }

    Ok((StatusCode::NO_CONTENT, [("Cache-Control", "no-store")]))
}

#[cfg(test)]
mod tests {
    use super::can_claim_more_governors;

    #[test]
    fn allows_claim_when_nine_governors_are_claimed() {
        assert!(can_claim_more_governors(9));
    }

    #[test]
    fn rejects_claim_when_ten_governors_are_claimed() {
        assert!(!can_claim_more_governors(10));
    }
}
