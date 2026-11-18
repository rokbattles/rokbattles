use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use rustc_hash::FxHashMap;

use self::{
    aggregate::aggregate_performance,
    detail::{aggregate_detail, aggregate_matchups},
    query::{parse_pairing_request, parse_pairings_request},
    store::{fetch_pairings_mails, fetch_pairings_overview},
    types::{PairingMatchupsResponse, PairingsRange, PairingsResponse},
};
use crate::{
    auth::AuthenticatedSession,
    error::ApiError,
    routes::governor::common::{ensure_governor_claim_for_user, parse_governor_id_param},
    state::AppState,
};

mod aggregate;
mod detail;
mod query;
mod store;
mod types;

/// Returns PvP totals grouped by commander pairing for a governor claimed by the caller.
pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    Query(params): Query<FxHashMap<String, String>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let request = parse_pairings_request(&params)?;

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let items = fetch_pairings_overview(
        &state,
        governor_id,
        &request.range,
        &request.exclude_activities,
        &request.exclude_battles,
    )
    .await?;
    let response = PairingsResponse {
        range: PairingsRange { start: request.range.start, end: request.range.end },
        items,
    };

    Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)))
}

/// Returns the pairing's PvP totals and all loadouts recorded in the requested date range.
pub async fn get_detail(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    Query(params): Query<FxHashMap<String, String>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let request = parse_pairing_request(&params)?;

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let mails = fetch_pairings_mails(
        &state,
        governor_id,
        &request.range,
        Some((request.primary_commander_id, request.secondary_commander_id)),
        &request.exclude_activities,
        &request.exclude_battles,
    )
    .await?;

    let response = aggregate_detail(
        &mails,
        &request.range,
        request.primary_commander_id,
        request.secondary_commander_id,
    );

    Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)))
}

/// Returns opponent pairing stats for all loadouts or for the requested loadout.
///
/// Includes only matchups with at least 100 battles.
pub async fn get_matchups(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    Query(params): Query<FxHashMap<String, String>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let request = parse_pairing_request(&params)?;

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let mails = fetch_pairings_mails(
        &state,
        governor_id,
        &request.range,
        Some((request.primary_commander_id, request.secondary_commander_id)),
        &request.exclude_activities,
        &request.exclude_battles,
    )
    .await?;

    let items = aggregate_matchups(
        &mails,
        &request.range,
        request.primary_commander_id,
        request.secondary_commander_id,
        request.loadout_key.as_deref(),
    );
    let response = PairingMatchupsResponse {
        range: PairingsRange { start: request.range.start, end: request.range.end },
        items,
    };

    Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)))
}

/// Returns daily and overall performance for the requested pairing and optional loadout.
pub async fn get_performance(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    Query(params): Query<FxHashMap<String, String>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let request = parse_pairing_request(&params)?;

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let mails = fetch_pairings_mails(
        &state,
        governor_id,
        &request.range,
        Some((request.primary_commander_id, request.secondary_commander_id)),
        &request.exclude_activities,
        &request.exclude_battles,
    )
    .await?;

    let response = aggregate_performance(
        &mails,
        &request.range,
        request.primary_commander_id,
        request.secondary_commander_id,
        request.loadout_key.as_deref(),
    );

    Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)))
}
