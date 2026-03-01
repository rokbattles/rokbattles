use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::{Json, http::StatusCode};

use crate::auth::AuthenticatedSession;
use crate::error::ApiError;
use crate::routes::governor::common::{ensure_governor_claim_for_user, parse_governor_id_param};
use crate::state::AppState;

use self::aggregate::aggregate_loot;
use self::query::parse_loot_request;
use self::store::{
    fetch_barbarian_battle_mails, fetch_barbarian_fort_mails, fetch_baulur_mails,
    fetch_marauder_battle_mails,
};
use self::types::LootResponse;

mod aggregate;
mod query;
mod store;
mod types;

/// Returns loot aggregates for a governor claimed by the current user.
pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let request = parse_loot_request(&params)?;

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let mail_receiver = format!("player_{governor_id}");
    let time_match = request.range.build_mail_time_match();

    let (barbarian_mails, marauder_mails, barbarian_fort_mails, baulur_mails) = tokio::try_join!(
        fetch_barbarian_battle_mails(&state, &mail_receiver, &time_match),
        fetch_marauder_battle_mails(&state, &mail_receiver, &time_match),
        fetch_barbarian_fort_mails(&state, &mail_receiver, &time_match),
        fetch_baulur_mails(&state, &mail_receiver, governor_id, &time_match),
    )?;

    let categories = aggregate_loot(
        barbarian_mails,
        marauder_mails,
        barbarian_fort_mails,
        baulur_mails,
        governor_id,
        &request.range,
    );
    let response = LootResponse::new(request.range.start, request.range.end, categories);

    Ok((
        StatusCode::OK,
        [("Cache-Control", "no-store")],
        Json(response),
    ))
}
