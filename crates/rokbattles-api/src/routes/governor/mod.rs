use std::sync::Arc;

use axum::Router;
use axum::routing::{patch, post};

use crate::state::AppState;

pub mod bind;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/bind", post(bind::post).delete(bind::delete))
        .route("/bind/default", patch(bind::patch_default))
}
