use std::sync::Arc;

use axum::{Router, routing::post};

use crate::state::AppState;

pub mod refresh_binds;

/// Routes used by cron jobs.
/// Requests must include `x-cron-secret`.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/refresh-binds", post(refresh_binds::post))
}
