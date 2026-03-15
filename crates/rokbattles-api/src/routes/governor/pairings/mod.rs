use std::{collections::HashMap, sync::Arc};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};

use self::{
    aggregate::{aggregate_loadouts, aggregate_opponents, aggregate_pairings},
    query::{
        parse_pairing_loadouts_request, parse_pairing_opponents_request, parse_pairings_request,
    },
    store::fetch_pairings_mails,
    types::{PairingLoadoutsResponse, PairingOpponentsResponse, PairingsRange, PairingsResponse},
};
use crate::{
    auth::AuthenticatedSession,
    error::ApiError,
    routes::governor::common::{ensure_governor_claim_for_user, parse_governor_id_param},
    state::AppState,
};

mod aggregate;
mod query;
mod store;
mod types;

/// Get aggregated pairing stats for a governor the current user has claimed.
pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let request = parse_pairings_request(&params)?;

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let mails = fetch_pairings_mails(&state, governor_id, &request.range, None).await?;
    let items = aggregate_pairings(&mails, &request.range);
    let response = PairingsResponse {
        range: PairingsRange { start: request.range.start, end: request.range.end },
        items,
    };

    Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)))
}

/// Get loadout-level aggregates for one selected commander pairing.
pub async fn get_loadouts(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let request = parse_pairing_loadouts_request(&params)?;

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let mails = fetch_pairings_mails(
        &state,
        governor_id,
        &request.range,
        Some(request.primary_commander_id),
    )
    .await?;
    let items = aggregate_loadouts(
        &mails,
        &request.range,
        request.primary_commander_id,
        request.secondary_commander_id,
        request.granularity,
    );
    let response = PairingLoadoutsResponse {
        range: PairingsRange { start: request.range.start, end: request.range.end },
        items,
    };

    Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)))
}

/// Get opponent aggregates for a selected pairing, optionally scoped to one loadout.
pub async fn get_opponents(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let request = parse_pairing_opponents_request(&params)?;

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let mails = fetch_pairings_mails(
        &state,
        governor_id,
        &request.range,
        Some(request.primary_commander_id),
    )
    .await?;
    let items = aggregate_opponents(
        &mails,
        &request.range,
        request.primary_commander_id,
        request.secondary_commander_id,
        request.granularity,
        request.loadout_key.as_deref(),
    );
    let response = PairingOpponentsResponse {
        range: PairingsRange { start: request.range.start, end: request.range.end },
        items,
    };

    Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)))
}
