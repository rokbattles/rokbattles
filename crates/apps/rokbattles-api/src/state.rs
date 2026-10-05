use std::sync::Arc;

use crate::db::{
    AuthRepository, GameLocalizationStore, GameQueryStore, ReportsStore, TerritoryPlannerV2Store,
};

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
    pub game_query: GameQueryStore,
    pub game_localizations: GameLocalizationStore,
    pub reports_store: ReportsStore,
    pub territory_planner_v2: TerritoryPlannerV2Store,
    pub discord_oauth: DiscordOAuthConfig,
    pub(crate) pk_token: Option<String>,
}

impl AppState {
    /// Create app state from the configured stores and OAuth settings.
    pub fn new(
        auth_store: Arc<dyn AuthRepository>,
        game_query: GameQueryStore,
        game_localizations: GameLocalizationStore,
        reports_store: ReportsStore,
        territory_planner_v2: TerritoryPlannerV2Store,
        discord_oauth: DiscordOAuthConfig,
        pk_token: Option<String>,
    ) -> Self {
        Self {
            auth_store,
            game_query,
            game_localizations,
            reports_store,
            territory_planner_v2,
            discord_oauth,
            pk_token,
        }
    }
}
