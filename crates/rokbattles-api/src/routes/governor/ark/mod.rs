use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::{Json, http::StatusCode};

use crate::auth::AuthenticatedSession;
use crate::error::ApiError;
use crate::routes::governor::common::ensure_governor_claim_for_user;
use crate::state::AppState;
use crate::time_utils::build_mail_time_match;

use self::mapper::{
    build_secondary_window, extract_mail_time_millis, extract_mail_times, map_match_detail,
    map_match_record,
};
use self::matcher::match_ark_mails;
use self::query::{parse_ark_list_request, parse_governor_id, parse_match_id};
use self::store::{
    fetch_ark_battle_info_mails, fetch_ark_battle_results_mail_by_id,
    fetch_ark_battle_results_mails, fetch_ark_individual_results_mails,
};
use self::types::{ArkDetailResponse, ArkHistoryResponse};

mod mapper;
mod matcher;
mod query;
mod store;
mod types;

const MATCH_DELTA_MILLIS: i64 = 60_000;

/// Returns Ark match history for a claimed governor.
pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id(&governor_id_raw)?;
    let request = parse_ark_list_request(&params)?;

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let mail_receiver = format!("player_{governor_id}");
    let battle_results =
        fetch_ark_battle_results_mails(&state, &mail_receiver, request.limit).await?;

    if battle_results.is_empty() {
        let response = ArkHistoryResponse {
            limit: request.limit,
            total: 0,
            items: Vec::new(),
        };

        return Ok((
            StatusCode::OK,
            [("Cache-Control", "no-store")],
            Json(response),
        ));
    }

    let primary_times = extract_mail_times(&battle_results);
    let (battle_info, individual_results) =
        match build_secondary_window(&primary_times, MATCH_DELTA_MILLIS) {
            Some(window) => {
                let time_match = build_mail_time_match(window.start_millis, window.end_millis);
                tokio::try_join!(
                    fetch_ark_battle_info_mails(&state, &mail_receiver, &time_match),
                    fetch_ark_individual_results_mails(&state, &mail_receiver, &time_match),
                )?
            }
            None => (Vec::new(), Vec::new()),
        };

    let matched = match_ark_mails(
        battle_results,
        battle_info,
        individual_results,
        MATCH_DELTA_MILLIS,
    );
    let items = matched
        .iter()
        .enumerate()
        .map(|(index, entry)| map_match_record(entry, index))
        .collect::<Vec<_>>();

    let response = ArkHistoryResponse {
        limit: request.limit,
        total: i64::try_from(items.len()).unwrap_or(i64::MAX),
        items,
    };

    Ok((
        StatusCode::OK,
        [("Cache-Control", "no-store")],
        Json(response),
    ))
}

/// Returns one Ark match detail record by mail ID for a claimed governor.
pub async fn get_by_id(
    State(state): State<Arc<AppState>>,
    Path((governor_id_raw, match_id_raw)): Path<(String, String)>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id(&governor_id_raw)?;
    let match_id = parse_match_id(&match_id_raw)?;

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let mail_receiver = format!("player_{governor_id}");
    let Some(battle_results) =
        fetch_ark_battle_results_mail_by_id(&state, &mail_receiver, &match_id).await?
    else {
        let response = ArkDetailResponse {
            id: match_id,
            ark_match: None,
        };

        return Ok((
            StatusCode::OK,
            [("Cache-Control", "no-store")],
            Json(response),
        ));
    };

    let Some(mail_time_millis) = extract_mail_time_millis(&battle_results) else {
        let response = ArkDetailResponse {
            id: match_id,
            ark_match: None,
        };

        return Ok((
            StatusCode::OK,
            [("Cache-Control", "no-store")],
            Json(response),
        ));
    };

    let time_match = build_mail_time_match(
        mail_time_millis.saturating_sub(MATCH_DELTA_MILLIS),
        mail_time_millis
            .saturating_add(MATCH_DELTA_MILLIS)
            .saturating_add(1),
    );
    let (battle_info, individual_results) = tokio::try_join!(
        fetch_ark_battle_info_mails(&state, &mail_receiver, &time_match),
        fetch_ark_individual_results_mails(&state, &mail_receiver, &time_match),
    )?;

    let matched = match_ark_mails(
        vec![battle_results],
        battle_info,
        individual_results,
        MATCH_DELTA_MILLIS,
    );
    let detail = matched.first().map(|entry| map_match_detail(entry, 0));

    let response = ArkDetailResponse {
        id: match_id,
        ark_match: detail,
    };

    Ok((
        StatusCode::OK,
        [("Cache-Control", "no-store")],
        Json(response),
    ))
}
