use std::sync::Arc;

use crate::db::{AuthRepository, ReportsStore};

/// Shared state used by route handlers.
#[derive(Clone)]
pub struct AppState {
    pub auth_store: Arc<dyn AuthRepository>,
    pub reports_store: ReportsStore,
    pub cron_secret: String,
}

impl AppState {
    /// Create app state from configured stores and secrets.
    pub fn new(
        auth_store: Arc<dyn AuthRepository>,
        reports_store: ReportsStore,
        cron_secret: String,
    ) -> Self {
        Self {
            auth_store,
            reports_store,
            cron_secret,
        }
    }
}
