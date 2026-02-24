use std::sync::Arc;

use crate::db::{AuthRepository, ReportsStore};

/// Shared application state for request handlers.
#[derive(Clone)]
pub struct AppState {
    pub auth_store: Arc<dyn AuthRepository>,
    pub reports_store: ReportsStore,
}

impl AppState {
    /// Create a new application state value.
    pub fn new(auth_store: Arc<dyn AuthRepository>, reports_store: ReportsStore) -> Self {
        Self {
            auth_store,
            reports_store,
        }
    }
}
