use std::sync::Arc;

use axum::{Router, routing::get};

use crate::state::AppState;

mod auth;
mod combat_lab;
mod game;
mod governor;
mod health;
mod loot_explorer;
mod reports;
mod territory_planner;

/// Build the top-level API router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health::get))
        .nest("/v1", v1_router())
        .nest("/v2", v2_router())
}

fn v2_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/global/combat-lab", get(combat_lab::get_pairing))
        .route("/global/combat-lab/rankings", get(combat_lab::get_rankings))
}

fn v1_router() -> Router<Arc<AppState>> {
    Router::new()
        .nest("/game", game::router())
        .nest("/governor", governor::router())
        .nest("/auth", auth::router())
        .route("/global/loot-explorer/barbarians", get(loot_explorer::get_barbarians))
        .route("/global/loot-explorer/barbarian-forts", get(loot_explorer::get_barbarian_forts))
        .route("/global/loot-explorer/baulurs", get(loot_explorer::get_baulurs))
        .route("/global/loot-explorer/karuak-ceremony", get(loot_explorer::get_karuak_ceremony))
        .route("/global/loot-explorer/kahars-treasure", get(loot_explorer::get_kahar_treasure))
        .route("/global/territory-planner/list", get(territory_planner::list))
        .route("/global/territory-planner/map/{map}", get(territory_planner::get_map))
        .route("/report/battle/{id}", get(reports::battle::get_by_id))
        .route("/report/duelbattle2/{id}", get(reports::duelbattle2::get_by_id))
        .route("/reports/battle", get(reports::battle::get))
        .route("/reports/duelbattle2", get(reports::duelbattle2::get))
}
