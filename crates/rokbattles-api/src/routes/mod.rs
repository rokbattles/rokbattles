use std::sync::Arc;

use axum::{Router, routing::get};

use crate::state::AppState;

mod auth;
mod governor;
mod health;
mod loot_explorer;
mod reports;

/// Build the top-level API router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/health", get(health::get)).nest("/v1", v1_router())
}

fn v1_router() -> Router<Arc<AppState>> {
    Router::new()
        .nest("/governor", governor::router())
        .nest("/auth", auth::router())
        .route("/global/loot-explorer/barbarians", get(loot_explorer::get_barbarians))
        .route("/global/loot-explorer/barbarian-forts", get(loot_explorer::get_barbarian_forts))
        .route("/global/loot-explorer/baulurs", get(loot_explorer::get_baulurs))
        .route("/report/battle/{id}", get(reports::battle::get_by_id))
        .route("/report/duelbattle2/{id}", get(reports::duelbattle2::get_by_id))
        .route("/reports/battle", get(reports::battle::get))
        .route("/reports/duelbattle2", get(reports::duelbattle2::get))
}
