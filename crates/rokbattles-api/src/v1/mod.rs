pub mod ingress;

use crate::AppState;
use axum::{Router, routing::post};
use tower_http::limit::RequestBodyLimitLayer;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ingress", post(ingress::ingress))
        .layer(RequestBodyLimitLayer::new(5 * 1024 * 1024)) // 5 MB
}
