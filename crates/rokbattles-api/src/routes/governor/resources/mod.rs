use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::{Json, http::StatusCode};

use crate::auth::AuthenticatedSession;
use crate::error::ApiError;
use crate::routes::governor::common::{ensure_governor_claim_for_user, parse_governor_id_param};
use crate::state::AppState;

use self::aggregate::aggregate_resources;
use self::query::parse_resources_request;
use self::store::fetch_resources_mails;
use self::types::{ResourcesRange, ResourcesResponse};

mod aggregate;
mod query;
mod store;
mod types;

/// Returns gathered resources for a governor claimed by the current user.
pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(governor_id_raw): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let request = parse_resources_request(&params)?;

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let mail_receiver = format!("player_{governor_id}");
    let time_match = request.range.build_mail_time_match();
    let mails = fetch_resources_mails(&state, &mail_receiver, &time_match).await?;

    let aggregated = aggregate_resources(mails, &request.range);
    let response = ResourcesResponse {
        range: ResourcesRange {
            start: request.range.start,
            end: request.range.end,
        },
        total_reports: aggregated.total_reports,
        crystals_gain: aggregated.crystals_gain,
        resources: aggregated.resources,
        daily: aggregated.daily,
    };

    Ok((
        StatusCode::OK,
        [("Cache-Control", "no-store")],
        Json(response),
    ))
}
