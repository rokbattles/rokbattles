use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

mod cron;
mod governor;
mod health;
mod reports;

/// Builds the top-level API router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health::get))
        .nest("/cron", cron::router())
        .nest("/v1", v1_router())
}

fn v1_router() -> Router<Arc<AppState>> {
    Router::new()
        .nest("/governor", governor::router())
        .route(
            "/report/duelbattle2/{id}",
            get(reports::duelbattle2::get_by_id),
        )
        .route("/reports/battle", get(reports::battle::get))
        .route("/reports/duelbattle2", get(reports::duelbattle2::get))
}
