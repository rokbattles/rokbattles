pub mod ingress;
pub mod reports;

use crate::AppState;
use axum::{Router, routing::get, routing::post};
use tower_http::limit::RequestBodyLimitLayer;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ingress", post(ingress::ingress))
        .route("/reports", get(reports::list_reports))
        .layer(RequestBodyLimitLayer::new(5 * 1024 * 1024)) // 5 MB
}
