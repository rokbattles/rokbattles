use std::{collections::HashMap, sync::Arc};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};

use self::{
    aggregate::{
        aggregate_personal_barbarian_loot, aggregate_personal_baulur_loot,
        aggregate_personal_fort_loot, aggregate_personal_kahar_treasure_loot,
    },
    query::{
        parse_barbarian_loot_request, parse_baulur_loot_request, parse_fort_loot_request,
        parse_kahar_treasure_loot_request,
    },
    store::{
        fetch_barbarian_battle_mails, fetch_barbarian_fort_mails, fetch_baulur_mails,
        fetch_kahar_treasure_mails, fetch_marauder_battle_mails, fetch_marauder_encampment_mails,
    },
    types::PersonalLootResponse,
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

/// Returns personal barbarian or marauder loot aggregates for a claimed governor.
pub async fn get_barbarians(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let request = parse_barbarian_loot_request(&params)?;

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let mail_receiver = format!("player_{governor_id}");
    let time_match = request.range.build_mail_time_match();
    let mails = match request.npc {
        query::BarbarianLootNpc::Barbarians => {
            fetch_barbarian_battle_mails(&state, &mail_receiver, &time_match).await?
        }
        query::BarbarianLootNpc::Marauders => {
            fetch_marauder_battle_mails(&state, &mail_receiver, &time_match).await?
        }
    };
    let groups = aggregate_personal_barbarian_loot(mails, &request);
    let response = PersonalLootResponse::new(request.range.start, request.range.end, groups);

    Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)))
}

/// Returns personal Kahar treasure loot aggregates for a claimed governor.
pub async fn get_kahars_treasure(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let request = parse_kahar_treasure_loot_request(&params)?;

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let mail_receiver = format!("player_{governor_id}");
    let time_match = request.range.build_mail_time_match();
    let mails = fetch_kahar_treasure_mails(&state, &mail_receiver, &time_match).await?;
    let groups = aggregate_personal_kahar_treasure_loot(mails, &request.range);
    let response = PersonalLootResponse::new(request.range.start, request.range.end, groups);

    Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)))
}

/// Returns personal barbarian fort or marauder encampment loot aggregates.
pub async fn get_barbarian_forts(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let request = parse_fort_loot_request(&params)?;

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let mail_receiver = format!("player_{governor_id}");
    let time_match = request.range.build_mail_time_match();
    let mails = match request.npc {
        query::FortLootNpc::BarbarianForts => {
            fetch_barbarian_fort_mails(&state, &mail_receiver, &time_match, request.level).await?
        }
        query::FortLootNpc::MarauderEncampments => {
            fetch_marauder_encampment_mails(&state, &mail_receiver, &time_match, request.level)
                .await?
        }
    };
    let groups = aggregate_personal_fort_loot(mails, &request);
    let response = PersonalLootResponse::new(request.range.start, request.range.end, groups);

    Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)))
}

/// Returns personal Baulur loot aggregates for a claimed governor.
pub async fn get_baulurs(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let request = parse_baulur_loot_request(&params)?;

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let mail_receiver = format!("player_{governor_id}");
    let time_match = request.range.build_mail_time_match();
    let npc_types: &[i64] = match request.npc {
        query::BaulurLootNpc::IronhandBaulur => &[102_000_055],
        query::BaulurLootNpc::MiserKhaolak => &[102_000_063],
    };
    let mails =
        fetch_baulur_mails(&state, &mail_receiver, governor_id, Some(npc_types), &time_match)
            .await?;
    let groups = aggregate_personal_baulur_loot(mails, governor_id, request.npc, &request.range);
    let response = PersonalLootResponse::new(request.range.start, request.range.end, groups);

    Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)))
}
