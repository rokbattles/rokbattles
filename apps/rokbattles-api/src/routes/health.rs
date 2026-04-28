use axum::{Json, response::IntoResponse};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

/// Simple health check endpoint.
pub async fn get() -> impl IntoResponse {
    Json(HealthResponse { status: "ok" })
}
