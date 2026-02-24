#![forbid(unsafe_code)]
//! ROK Battles API service.
//!
//! This crate hosts the standalone API service used by the platform.
//! Route handlers are split one file per route under `routes/`, while shared
//! auth and persistence logic live in dedicated modules to keep endpoint files thin.

pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod routes;
pub mod state;

use std::sync::Arc;

use axum::Router;

use crate::state::AppState;

/// Build the API router with shared application state.
pub fn build_router(state: Arc<AppState>) -> Router {
    routes::router().with_state(state)
}
