use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use futures::{StreamExt, TryStreamExt, stream};
use mongodb::bson::doc;
use rokbattles_bson::bson_to_i64_exact;
use rustc_hash::FxHashSet;

use super::{
    NO_STORE, selected_ids,
    store::{
        database_error, find_primary_claim, insert_claim, preserve_existing_default,
        resolve_claim_app_uid,
    },
    types::{BindSelection, BindSuggestion},
};
use crate::{
    db::{ReportsStore, exclude_test_reports_filter},
    error::ApiError,
    routes::{
        governor::{common::parse_governor_id_param, snapshot::find_latest_sender_snapshot},
        me::AccountAccess,
    },
    state::AppState,
};

/// Lists available governors with the same Lilith ID as the caller's primary bind.
pub(crate) async fn suggestions(
    State(state): State<Arc<AppState>>,
    Path(governor_id): Path<String>,
    access: AccountAccess,
) -> Result<impl IntoResponse, ApiError> {
    access.require_session()?;
    let governor_id = parse_governor_id_param(&governor_id)?;

    let store = &state.reports_store;
    let app_uid = primary_app_uid(store, access.account_id(), governor_id).await?;
    let battle = store.battle_collection();

    let ids: Vec<i64> = battle
        .distinct(
            "sender.player_id",
            doc! { "$and": [
                exclude_test_reports_filter(),
                { "sender.app_uid": app_uid, "sender.player_id": { "$gt": 0, "$ne": governor_id } },
            ] },
        )
        .await
        .map_err(database_error)?
        .iter()
        .filter_map(bson_to_i64_exact)
        .collect();

    let claimed: FxHashSet<i64> = store
        .claimed_governors_collection()
        .distinct("governorId", doc! { "governorId": { "$in": &ids } })
        .await
        .map_err(database_error)?
        .iter()
        .filter_map(bson_to_i64_exact)
        .collect();

    // Check the latest snapshot because a match may come from an older report.
    let mut suggestions: Vec<BindSuggestion> =
        stream::iter(ids.into_iter().filter(|id| !claimed.contains(id)).map(|id| async move {
            let snapshot = find_latest_sender_snapshot(battle, id).await?;
            Ok(snapshot.filter(|snapshot| snapshot.app_uid == Some(app_uid)).map(|snapshot| {
                BindSuggestion {
                    governor_id: id,
                    governor_name: snapshot.governor_name,
                    governor_avatar: snapshot.governor_avatar,
                }
            }))
        }))
        .buffer_unordered(12)
        .try_filter_map(|suggestion| async { Ok(suggestion) })
        .try_collect()
        .await?;

    suggestions.sort_unstable_by_key(|suggestion| suggestion.governor_id);
    Ok((NO_STORE, Json(suggestions)))
}

/// Binds the selected governors after checking that they still share the primary bind's Lilith ID.
pub(crate) async fn bind_suggestions(
    State(state): State<Arc<AppState>>,
    Path(governor_id): Path<String>,
    access: AccountAccess,
    Json(selection): Json<BindSelection>,
) -> Result<impl IntoResponse, ApiError> {
    access.require_session()?;
    let governor_id = parse_governor_id_param(&governor_id)?;
    let ids = selected_ids(selection.governor_ids)?;
    let discord_id = access.account_id();

    let _operation = state.account_operations.lock(discord_id).await;
    access.ensure_active(&state).await?;

    let store = &state.reports_store;
    let app_uid = primary_app_uid(store, discord_id, governor_id).await?;
    let battle = store.battle_collection();
    let claims = store.claimed_governors_collection();

    let candidates: Vec<_> = stream::iter(ids.into_iter().map(|id| async move {
        let snapshot = find_latest_sender_snapshot(battle, id)
            .await?
            .filter(|snapshot| snapshot.app_uid == Some(app_uid))
            .ok_or_else(|| ApiError::bad_request("Select governors from this Lilith account."))?;

        if let Some(existing) =
            claims.find_one(doc! { "governorId": id }).await.map_err(database_error)?
        {
            if existing.get_str("discordId").ok() == Some(discord_id) {
                return Ok(None);
            }

            return Err(ApiError::conflict("A selected governor was already claimed."));
        }

        Ok(Some((id, snapshot)))
    }))
    .buffered(12)
    .try_filter_map(|candidate| async { Ok(candidate) })
    .try_collect()
    .await?;

    if !candidates.is_empty() {
        preserve_existing_default(store, discord_id).await?;
    }

    for (id, snapshot) in candidates {
        insert_claim(store, discord_id, id, app_uid, &snapshot, false).await?;
    }

    Ok((StatusCode::NO_CONTENT, NO_STORE))
}

async fn primary_app_uid(
    store: &ReportsStore,
    discord_id: &str,
    governor_id: i64,
) -> Result<i64, ApiError> {
    let claim = find_primary_claim(store, discord_id, governor_id).await?;

    resolve_claim_app_uid(store, &claim).await?.ok_or_else(|| ApiError::conflict(
        "This governor's Lilith ID is unavailable. Upload a battle report for this governor and try again once it has been processed.",
    ))
}
