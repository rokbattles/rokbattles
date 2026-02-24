use axum::Json;
use axum::response::IntoResponse;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

/// Liveness endpoint.
pub async fn get() -> impl IntoResponse {
    Json(HealthResponse { status: "ok" })
}
