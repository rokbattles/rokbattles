use std::sync::Arc;

use crate::{
    auth::account_operations::AccountOperations,
    db::{
        AuthRepository, DeveloperAppsStore, GameLocalizationStore, GameQueryStore,
        ProgramApplicationsStore, ReportsStore, TerritoryPlannerV2Store,
    },
    oidc::Oidc,
    turnstile::Turnstile,
};

/// Discord OAuth settings used by auth routes.
#[derive(Clone)]
pub struct DiscordOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

/// Storage and token verification used by the program application routes.
#[derive(Clone)]
pub(crate) struct ProgramApplicationsState {
    pub(crate) store: ProgramApplicationsStore,
    pub(crate) turnstile: Turnstile,
}

/// Shared state used by route handlers.
#[derive(Clone)]
pub struct AppState {
    pub(crate) account_operations: AccountOperations,
    pub auth_store: Arc<dyn AuthRepository>,
    pub game_query: GameQueryStore,
    pub game_localizations: GameLocalizationStore,
    pub reports_store: ReportsStore,
    pub territory_planner_v2: TerritoryPlannerV2Store,
    pub discord_oauth: DiscordOAuthConfig,
    #[cfg(test)]
    pub(crate) test_discord_identity: Option<crate::db::DiscordUserUpsert>,
    pub(crate) pk_token: Option<String>,
    pub(crate) kdmt_token: Option<String>,
    pub(crate) oidc: Option<Arc<Oidc>>,
    pub(crate) developer_apps: Option<DeveloperAppsStore>,
    pub(crate) program_applications: Option<ProgramApplicationsState>,
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
            account_operations: AccountOperations::default(),
            auth_store,
            game_query,
            game_localizations,
            reports_store,
            territory_planner_v2,
            discord_oauth,
            #[cfg(test)]
            test_discord_identity: None,
            pk_token,
            kdmt_token: None,
            program_applications: None,
            developer_apps: None,
            oidc: None,
        }
    }

    pub fn with_kdmt_token(mut self, token: Option<String>) -> Self {
        self.kdmt_token = token;
        self
    }

    /// Enables the private OIDC provider.
    pub fn with_oidc(mut self, oidc: Oidc) -> Self {
        self.oidc = Some(Arc::new(oidc));
        self
    }

    /// Enables developer app registration and settings routes.
    pub fn with_developer_apps(mut self, store: DeveloperAppsStore) -> Self {
        self.developer_apps = Some(store);
        self
    }

    /// Enables program application routes with the given store and verifier.
    pub fn with_program_applications(
        mut self,
        store: ProgramApplicationsStore,
        turnstile: Turnstile,
    ) -> Self {
        self.program_applications = Some(ProgramApplicationsState { store, turnstile });
        self
    }
}
