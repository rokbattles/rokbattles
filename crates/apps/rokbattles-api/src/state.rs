use std::sync::Arc;

use crate::db::{AuthRepository, DnsCheckStore, GameLocalizationStore, ReportsStore};

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
    pub game_localizations: GameLocalizationStore,
    pub reports_store: ReportsStore,
    pub dns_check_store: DnsCheckStore,
    pub dns_check_secret: String,
    pub discord_oauth: DiscordOAuthConfig,
}

impl AppState {
    /// Create app state from the configured stores and OAuth settings.
    pub fn new(
        auth_store: Arc<dyn AuthRepository>,
        game_localizations: GameLocalizationStore,
        reports_store: ReportsStore,
        dns_check_store: DnsCheckStore,
        dns_check_secret: String,
        discord_oauth: DiscordOAuthConfig,
    ) -> Self {
        Self {
            auth_store,
            game_localizations,
            reports_store,
            dns_check_store,
            dns_check_secret,
            discord_oauth,
        }
    }
}
