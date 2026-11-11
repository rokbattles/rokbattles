#![forbid(unsafe_code)]
//! ROK Battles API service.

pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod rate_limit;
pub mod routes;
pub mod state;
pub(crate) mod time_utils;

use std::sync::Arc;

use axum::Router;

use crate::{rate_limit::RateLimitMode, state::AppState};

pub fn build_router(state: Arc<AppState>, rate_limit_mode: RateLimitMode) -> Router {
    routes::router(rate_limit_mode).with_state(state)
}
