use std::sync::Arc;

use axum::Router;
use axum::routing::post;

use crate::state::AppState;

pub mod refresh_binds;

/// Cron-only routes.
/// Requests must include `x-cron-secret`.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/refresh-binds", post(refresh_binds::post))
}
