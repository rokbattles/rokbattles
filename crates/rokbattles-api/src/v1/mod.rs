pub mod ingress;
pub mod report;
pub mod reports;

use crate::AppState;
use axum::{Router, routing::get, routing::post};
use tower_http::limit::RequestBodyLimitLayer;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ingress", post(ingress::ingress))
        .route("/reports", get(reports::list_reports))
        .route("/report/{parent_hash}", get(report::report_by_parent))
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)) // 10 MB
}
