use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::{Json, http::StatusCode};
use mongodb::bson::doc;

use crate::auth::AuthenticatedSession;
use crate::error::ApiError;
use crate::state::AppState;

use self::aggregate::aggregate_loot;
use self::query::{parse_governor_id, parse_loot_request};
use self::store::{fetch_barbarian_battle_mails, fetch_barbarian_fort_mails, fetch_baulur_mails};
use self::types::LootResponse;

mod aggregate;
mod query;
mod store;
mod types;

/// Return loot summary for a claimed governor.
pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id(&governor_id_raw)?;
    let request = parse_loot_request(&params)?;

    let claim = state
        .reports_store
        .claimed_governors_collection()
        .find_one(doc! {
            "discordId": &session.user.discord_id,
            "governorId": governor_id
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    if claim.is_none() {
        return Err(ApiError::not_found("Claim not found"));
    }

    let mail_receiver = format!("player_{governor_id}");
    let time_match = request.range.build_mail_time_match();

    let (barbarian_mails, barbarian_fort_mails, baulur_mails) = tokio::try_join!(
        fetch_barbarian_battle_mails(&state, &mail_receiver, &time_match),
        fetch_barbarian_fort_mails(&state, &mail_receiver, &time_match),
        fetch_baulur_mails(&state, &mail_receiver, governor_id, &time_match),
    )?;

    let categories = aggregate_loot(
        barbarian_mails,
        barbarian_fort_mails,
        baulur_mails,
        governor_id,
        &request.range,
    );
    let response = LootResponse::new(
        request.range.start.clone(),
        request.range.end.clone(),
        categories,
    );

    Ok((
        StatusCode::OK,
        [("Cache-Control", "no-store")],
        Json(response),
    ))
}
