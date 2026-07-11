use std::sync::Arc;

use crate::db::{AuthRepository, ReportsStore};

/// Discord OAuth settings used by auth routes.
#[derive(Clone)]
pub struct DiscordOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

/// Shared state used by route handlers.
#[derive(Clone)]
pub struct AppState {
    pub auth_store: Arc<dyn AuthRepository>,
    pub reports_store: ReportsStore,
    pub discord_oauth: DiscordOAuthConfig,
}

impl AppState {
    /// Create app state from the configured stores and OAuth settings.
    pub fn new(
        auth_store: Arc<dyn AuthRepository>,
        reports_store: ReportsStore,
        discord_oauth: DiscordOAuthConfig,
    ) -> Self {
        Self { auth_store, reports_store, discord_oauth }
    }
}
