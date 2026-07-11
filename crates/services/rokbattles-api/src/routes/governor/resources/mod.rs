use std::{collections::HashMap, sync::Arc};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};

use self::{
    aggregate::aggregate_resources,
    query::parse_resources_request,
    store::fetch_resources_mails,
    types::{ResourcesRange, ResourcesResponse},
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
        range: ResourcesRange { start: request.range.start, end: request.range.end },
        total_reports: aggregated.total_reports,
        crystals_gain: aggregated.crystals_gain,
        resources: aggregated.resources,
        daily: aggregated.daily,
    };

    Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)))
}
