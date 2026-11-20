//! Routes for governor binds, shares, and related-governor discovery.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use mongodb::bson::{DateTime, Document, doc, oid::ObjectId};
use rustc_hash::FxHashSet;

use self::{
    store::{
        database_error, existing_primary_bind, find_primary_claim, insert_claim,
        owned_lilith_accounts, preserve_existing_default, primary_filter, set_default,
        within_lilith_quota,
    },
    types::{BindGovernorResponse, BindSelection, ShareSelection},
};
use crate::{
    db::{ReportsStore, is_duplicate_key},
    error::ApiError,
    routes::{
        governor::{common::parse_governor_id_param, snapshot::find_latest_sender_snapshot},
        me::AccountAccess,
    },
    state::AppState,
};

mod discovery;
mod store;
mod types;

pub(crate) use discovery::{bind_suggestions, suggestions};
pub(crate) use store::{find_accessible_claim, load_claimed_governors, share_documents, share_for};
pub(crate) use types::ClaimedGovernor;

const NO_STORE: [(&str, &str); 1] = [("Cache-Control", "no-store")];

/// Creates a primary bind for the authenticated account.
///
/// Battle reports supply optional governor details. The limit counts distinct
/// known Lilith IDs across the account's primary binds.
pub async fn post(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    access: AccountAccess,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let discord_id = access.account_id();

    let _operation = state.account_operations.lock(discord_id).await;
    access.ensure_active(&state).await?;

    let mut claim = claim_governor(&state.reports_store, discord_id, governor_id).await?;
    claim.shared_with.clear();
    Ok((NO_STORE, Json(BindGovernorResponse { claim })))
}

async fn claim_governor(
    store: &ReportsStore,
    discord_id: &str,
    governor_id: i64,
) -> Result<ClaimedGovernor, ApiError> {
    if let Some(claim) = existing_primary_bind(store, discord_id, governor_id).await? {
        return Ok(claim);
    }

    let snapshot = find_latest_sender_snapshot(store.battle_collection(), governor_id)
        .await?
        .unwrap_or_default();
    if let Some(app_uid) = snapshot.app_uid
        && !within_lilith_quota(&owned_lilith_accounts(store, discord_id).await?, app_uid)
    {
        return Err(ApiError::bad_request(
            "You can bind governors from up to 100 Lilith accounts.",
        ));
    }

    let mut default = !preserve_existing_default(store, discord_id).await?;
    while let Err(error) = insert_claim(store, discord_id, governor_id, &snapshot, default).await {
        if !is_duplicate_key(&error) {
            return Err(database_error(error));
        }
        if let Some(claim) = existing_primary_bind(store, discord_id, governor_id).await? {
            return Ok(claim);
        }
        if !default {
            return Err(database_error(error));
        }
        // Another API instance may have inserted this account's first bind.
        default = false;
    }

    Ok(ClaimedGovernor {
        governor_id,
        governor_name: snapshot.governor_name,
        governor_avatar: snapshot.governor_avatar,
        default,
        is_shared: false,
        shared_with: Vec::new(),
    })
}

/// Sets the caller's default governor using one of their primary or shared binds.
pub async fn patch_default(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    access: AccountAccess,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;

    let _operation = state.account_operations.lock(access.account_id()).await;
    access.ensure_active(&state).await?;

    let store = &state.reports_store;
    let claim = access.governor_claim(&state, governor_id).await?;
    set_default(store, access.account_id(), &claim).await?;

    Ok((StatusCode::NO_CONTENT, NO_STORE))
}

/// Removes the caller's bind. Removing a primary bind also revokes its shares.
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    access: AccountAccess,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    unbind(&state, &access, &[governor_id]).await?;

    Ok((StatusCode::NO_CONTENT, NO_STORE))
}

pub(crate) async fn bulk_unbind(
    State(state): State<Arc<AppState>>,
    access: AccountAccess,
    Json(selection): Json<BindSelection>,
) -> Result<impl IntoResponse, ApiError> {
    access.require_session()?;
    let ids = selected_ids(selection.governor_ids)?;
    unbind(&state, &access, &ids).await?;

    Ok((StatusCode::NO_CONTENT, NO_STORE))
}

async fn unbind(
    state: &AppState,
    access: &AccountAccess,
    governor_ids: &[i64],
) -> Result<(), ApiError> {
    let discord_id = access.account_id();
    let _operation = state.account_operations.lock(discord_id).await;
    access.ensure_active(state).await?;

    let claims = state.reports_store.claimed_governors_collection();
    let mut owned = Vec::new();
    let mut shared = Vec::new();

    for &id in governor_ids {
        let claim = access.governor_claim(state, id).await?;
        if let Some(share_id) =
            share_for(&claim, discord_id).and_then(|share| share.get_object_id("_id").ok())
        {
            shared.push(share_id);
        } else if let Some(id) = claim.get("_id") {
            owned.push(id.clone());
        }
    }

    if !owned.is_empty() {
        claims
            .delete_many(doc! { "_id": { "$in": owned }, "discordId": discord_id })
            .await
            .map_err(database_error)?;
    }

    if !shared.is_empty() {
        claims
            .update_many(
                doc! { "shares._id": { "$in": &shared } },
                doc! { "$pull": { "shares": { "_id": { "$in": shared }, "discordId": discord_id } } },
            )
            .await
            .map_err(database_error)?;
    }

    Ok(())
}

/// Shares the selected primary binds with one Discord account.
pub(crate) async fn share_selection(
    State(state): State<Arc<AppState>>,
    access: AccountAccess,
    Json(selection): Json<ShareSelection>,
) -> Result<impl IntoResponse, ApiError> {
    let ids = selected_ids(selection.governor_ids)?;
    let recipient = parse_discord_id(&selection.discord_id)?;
    let owner = access.account_id();

    if recipient == owner {
        return Err(ApiError::bad_request("Enter another user's Discord ID."));
    }

    let _operation = state.account_operations.lock(owner).await;
    access.ensure_active(&state).await?;

    let store = &state.reports_store;
    let mut claims = Vec::with_capacity(ids.len());
    for governor_id in ids {
        claims.push(find_primary_claim(store, owner, governor_id).await?);
    }

    for claim in claims {
        add_share(store.claimed_governors_collection(), &claim, recipient).await?;
    }

    Ok((StatusCode::NO_CONTENT, NO_STORE))
}

async fn add_share(
    claims: &mongodb::Collection<Document>,
    claim: &Document,
    recipient: &str,
) -> Result<(), ApiError> {
    claims
        .update_one(
            doc! { "_id": claim.get("_id"), "shares.discordId": { "$ne": recipient } },
            doc! { "$push": { "shares": {
                "_id": ObjectId::new(),
                "discordId": recipient,
                "createdAt": DateTime::now(),
            } } },
        )
        .await
        .map_err(database_error)?;

    Ok(())
}

/// Revokes one recipient's access to a governor the caller holds.
pub(crate) async fn revoke_share(
    State(state): State<Arc<AppState>>,
    Path((governor_id, recipient)): Path<(String, String)>,
    access: AccountAccess,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id)?;
    let recipient = parse_discord_id(&recipient)?;
    let owner = access.account_id();

    let _operation = state.account_operations.lock(owner).await;
    access.ensure_active(&state).await?;

    let result = state
        .reports_store
        .claimed_governors_collection()
        .update_one(
            primary_filter(owner, governor_id),
            doc! { "$pull": { "shares": { "discordId": recipient } } },
        )
        .await
        .map_err(database_error)?;

    if result.matched_count != 1 {
        return Err(ApiError::not_found("Primary bind not found"));
    }

    Ok((StatusCode::NO_CONTENT, NO_STORE))
}

fn selected_ids(ids: Vec<i64>) -> Result<Vec<i64>, ApiError> {
    if ids.is_empty() || ids.iter().any(|id| *id <= 0) {
        return Err(ApiError::bad_request("Select one or more valid governors."));
    }

    let mut seen = FxHashSet::default();
    Ok(ids.into_iter().filter(|id| seen.insert(*id)).collect())
}

fn parse_discord_id(raw: &str) -> Result<&str, ApiError> {
    let id = raw.trim();
    if !matches!(id.parse::<u64>(), Ok(value) if value > 0)
        || id.starts_with('0')
        || !id.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(ApiError::bad_request("Enter a valid Discord ID."));
    }

    Ok(id)
}
