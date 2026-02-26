use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

mod cron;
mod governor;
mod health;
mod reports;

/// Build top-level API routes.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health::get))
        .nest("/cron", cron::router())
        .nest("/v1", v1_router())
}

fn v1_router() -> Router<Arc<AppState>> {
    Router::new()
        .nest("/governor", governor::router())
        .route("/reports/battle", get(reports::battle::get))
        .route("/reports/duelbattle2", get(reports::duelbattle2::get))
}
