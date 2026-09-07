use std::sync::Arc;

use axum::{Router, routing::get};

use crate::state::AppState;

mod query;
mod translate;

pub(super) const DEFAULT_VERSION: &str = "1.1.11.25";

/// Build game routes.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/translate", get(translate::get).post(translate::post))
        .route("/query", get(query::get).post(query::post))
}
