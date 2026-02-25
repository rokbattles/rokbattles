#![forbid(unsafe_code)]
//! ROK Battles API service.

pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod routes;
pub mod state;

use std::sync::Arc;

use axum::Router;

use crate::state::AppState;

/// Build the root router with shared app state.
pub fn build_router(state: Arc<AppState>) -> Router {
    routes::router().with_state(state)
}
