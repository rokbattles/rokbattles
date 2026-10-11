//! HTTP routing and endpoint-specific request validation.

mod headers;
mod relay;
mod security;
mod upload;

use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::StatusCode,
    middleware,
    routing::{get, post},
};
use tower_http::limit::RequestBodyLimitLayer;

use crate::state::AppState;

const MAX_REQUEST_BODY_BYTES: usize = 25 * 1024 * 1024;

/// Builds the router with a 25 MiB request body limit for both upload endpoints.
pub(crate) fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v2/upload", post(upload::upload))
        .route("/v2/relay/upload", post(relay::upload))
        .with_state(state)
        // Use one request-wide limiter; it eagerly rejects a known oversized Content-Length and
        // caps streamed bodies. Disabling the extractor wrapper avoids nesting two limiters.
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(MAX_REQUEST_BODY_BYTES))
        // Keep response hardening outermost so extractor, routing, and body-limit errors get it.
        .layer(middleware::from_fn(security::response_headers))
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
