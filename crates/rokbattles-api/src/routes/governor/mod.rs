use std::sync::Arc;

use axum::Router;
use axum::routing::{get, patch, post};

use crate::state::AppState;

pub mod bind;
pub(crate) mod common;
pub mod loot;
pub(crate) mod snapshot;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{governor_id}/bind", post(bind::post).delete(bind::delete))
        .route("/{governor_id}/bind/default", patch(bind::patch_default))
        .route("/{governor_id}/loot", get(loot::get))
}
