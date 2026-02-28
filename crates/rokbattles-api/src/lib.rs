#![forbid(unsafe_code)]
//! ROK Battles API service.

pub mod auth;
pub(crate) mod bson_utils;
pub mod config;
pub mod db;
pub mod error;
pub mod routes;
pub mod state;
pub(crate) mod time_utils;

use std::sync::Arc;

use axum::Router;

use crate::state::AppState;

/// Build the root router and attach shared app state.
pub fn build_router(state: Arc<AppState>) -> Router {
    routes::router().with_state(state)
}
