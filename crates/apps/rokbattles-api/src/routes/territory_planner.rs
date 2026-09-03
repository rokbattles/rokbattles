//! Public Territory Planner configuration endpoints.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;

use crate::{db::TerritoryPlannerMapSummary, error::ApiError, state::AppState};

const CACHE_CONTROL: [(&str, &str); 1] = [("Cache-Control", "public, max-age=3600")];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerritoryPlannerListResponse {
    maps: Vec<TerritoryPlannerMapSummary>,
}

/// Return the Territory Planner map catalog.
pub async fn list(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, ApiError> {
    let maps = state
        .territory_planner
        .list_maps()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    Ok((StatusCode::OK, CACHE_CONTROL, Json(TerritoryPlannerListResponse { maps })))
}

/// Return one map's complete Territory Planner configuration.
pub async fn get_map(
    State(state): State<Arc<AppState>>,
    Path(map): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let config = state
        .territory_planner
        .find_map(&map)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("territory planner map not found"))?;
    Ok((StatusCode::OK, CACHE_CONTROL, Json(config)))
}
