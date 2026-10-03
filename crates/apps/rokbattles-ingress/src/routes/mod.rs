//! HTTP routing and endpoint-specific request validation.

mod headers;
mod relay;
mod upload;

use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::StatusCode,
    routing::{get, post},
};

use crate::state::AppState;

/// Builds the router with a 25 MiB request body limit for both upload endpoints.
pub(crate) fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v2/upload", post(upload::upload))
        .route("/v2/relay/upload", post(relay::upload))
        .with_state(state)
        .layer(DefaultBodyLimit::max(25 * 1024 * 1024))
}

/// Liveness only; does not probe MongoDB or ClamAV.
async fn health() -> StatusCode {
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        extract::{FromRequest, Multipart},
        http::Request,
    };

    pub(super) async fn multipart(body: &str) -> Multipart {
        let request = Request::builder()
            .header("content-type", "multipart/form-data; boundary=mail-boundary")
            .body(Body::from(body.to_string()))
            .expect("multipart request");

        Multipart::from_request(request, &()).await.expect("multipart extractor")
    }
}
