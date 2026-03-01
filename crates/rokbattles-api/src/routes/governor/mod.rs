use std::sync::Arc;

use axum::Router;
use axum::routing::{get, patch, post};

use crate::state::AppState;

pub mod ark;
pub mod bind;
pub(crate) mod common;
pub(crate) mod date_range;
pub mod loot;
pub mod pairings;
pub mod resources;
pub(crate) mod snapshot;
pub(crate) mod store_utils;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{governor_id}/bind", post(bind::post).delete(bind::delete))
        .route("/{governor_id}/bind/default", patch(bind::patch_default))
        .route("/{governor_id}/ark", get(ark::get))
        .route("/{governor_id}/ark/{match_id}", get(ark::get_by_id))
        .route("/{governor_id}/loot", get(loot::get))
        .route("/{governor_id}/resources", get(resources::get))
        .route("/{governor_id}/pairings", get(pairings::get))
        .route(
            "/{governor_id}/pairings/loadouts",
            get(pairings::get_loadouts),
        )
        .route(
            "/{governor_id}/pairings/opponents",
            get(pairings::get_opponents),
        )
}
