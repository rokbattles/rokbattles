use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use futures::{StreamExt, TryStreamExt, stream};
use rustc_hash::FxHashMap;

use self::{
    mapper::{
        build_secondary_window, extract_mail_time_millis, extract_mail_times, map_match_detail,
        map_match_record,
    },
    matcher::match_ark_mails,
    query::{parse_ark_list_request, parse_match_id},
    store::{
        fetch_ark_battle_results_mail_by_id, fetch_ark_battle_results_mails,
        fetch_ark_individual_results_mails,
    },
    types::{ArkDetailResponse, ArkHistoryResponse, ArkLeague},
};
use crate::{
    auth::AuthenticatedSession,
    error::ApiError,
    routes::governor::common::{ensure_governor_claim_for_user, parse_governor_id_param},
    state::AppState,
    time_utils::build_mail_time_match,
};

mod mapper;
mod matcher;
mod query;
pub(crate) mod reports;
mod store;
mod types;

const MATCH_DELTA_MILLIS: i64 = 60_000;

/// Returns Ark match history for a claimed governor.
pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    Query(params): Query<FxHashMap<String, String>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let request = parse_ark_list_request(&params);

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let mail_receiver = format!("player_{governor_id}");
    let battle_results =
        fetch_ark_battle_results_mails(&state, &mail_receiver, request.limit).await?;

    if battle_results.is_empty() {
        let response = ArkHistoryResponse { limit: request.limit, total: 0, items: Vec::new() };

        return Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)));
    }

    let primary_times = extract_mail_times(&battle_results);
    let individual_results = match build_secondary_window(&primary_times, MATCH_DELTA_MILLIS) {
        Some(window) => {
            let time_match = build_mail_time_match(window.start_millis, window.end_millis);
            fetch_ark_individual_results_mails(&state, &mail_receiver, &time_match).await?
        }
        None => Vec::new(),
    };

    let matched = match_ark_mails(battle_results, individual_results, MATCH_DELTA_MILLIS);
    let summaries: Vec<_> =
        matched.iter().enumerate().map(|(index, entry)| map_match_record(entry, index)).collect();

    let mut items = stream::iter(summaries.into_iter().map(|mut summary| {
        let state = Arc::clone(&state);
        let receiver = mail_receiver.clone();

        async move {
            if summary.league == ArkLeague::Unknown {
                let scope = reports::resolve_report_scope(&state, &receiver, &summary).await?;
                if let Some(league) = scope.league {
                    summary.league = league;
                }
            }

            Ok::<_, ApiError>(summary)
        }
    }))
    .buffered(6)
    .try_collect::<Vec<_>>()
    .await?;

    items.retain(|item| {
        matches!(item.league, ArkLeague::Golden | ArkLeague::Silver | ArkLeague::Osiris)
    });

    let response = ArkHistoryResponse {
        limit: request.limit,
        total: i64::try_from(items.len()).unwrap_or(i64::MAX),
        items,
    };

    Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)))
}

/// Returns one Ark match detail record by mail ID for a claimed governor.
pub async fn get_by_id(
    State(state): State<Arc<AppState>>,
    Path((governor_id_raw, match_id_raw)): Path<(String, String)>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let match_id = parse_match_id(&match_id_raw)?;

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let mail_receiver = format!("player_{governor_id}");
    let Some(battle_results) =
        fetch_ark_battle_results_mail_by_id(&state, &mail_receiver, &match_id).await?
    else {
        let response = ArkDetailResponse { id: match_id, ark_match: None };

        return Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)));
    };

    let Some(mail_time_millis) = extract_mail_time_millis(&battle_results) else {
        let response = ArkDetailResponse { id: match_id, ark_match: None };

        return Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)));
    };

    let time_match = build_mail_time_match(
        mail_time_millis.saturating_sub(MATCH_DELTA_MILLIS),
        mail_time_millis.saturating_add(MATCH_DELTA_MILLIS).saturating_add(1),
    );
    let individual_results =
        fetch_ark_individual_results_mails(&state, &mail_receiver, &time_match).await?;

    let matched = match_ark_mails(vec![battle_results], individual_results, MATCH_DELTA_MILLIS);
    let mut detail = matched.first().map(|entry| map_match_detail(entry, 0));

    if let Some(detail) = detail.as_mut() {
        let scope = reports::resolve_report_scope(&state, &mail_receiver, &detail.summary).await?;

        if let Some(league) = scope.league {
            detail.summary.league = league;
        }

        detail.battle_reports = scope.coverage;
    }

    let response = ArkDetailResponse { id: match_id, ark_match: detail };

    Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)))
}
