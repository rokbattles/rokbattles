//! Database helpers used by API routes.

mod auth_store;

mod game_localization_store;
mod game_query_store;
mod program_applications_store;
mod reports_store;
mod territory_planner_v2_store;

pub use auth_store::{
    AuthRepository, AuthStoreError, DesktopLogin, DesktopOAuthState, DiscordUserUpsert,
    ListedSessionRecord, MongoAuthStore, NewSessionRecord, OAuthStateRecord, SessionPlatform,
    SessionRecord, UserRecord,
};
pub use game_localization_store::{
    GameLocalizationStore, GameLocalizationStoreError, GameTranslation,
};
pub use game_query_store::{
    GameQueryColumn, GameQueryRepository, GameQuerySheet, GameQueryStore, GameQueryStoreError,
};
pub use program_applications_store::{
    FormAnswer, PROGRAM_APPLICATIONS_COLLECTION, Program, ProgramApplication,
    ProgramApplicationsStore,
};
pub use reports_store::{
    ReportsStore, TEST_CLIENT_APP_ID, exclude_test_client_filter, exclude_test_reports_filter,
};
pub use territory_planner_v2_store::{
    TerritoryAsset, TerritoryBuildingRule, TerritoryCollisionRule, TerritoryCostTier,
    TerritoryMapCatalog, TerritoryMapSummary, TerritoryPlanSnapshot, TerritoryPlannerV2Store,
    TerritoryResourceCost, TerritoryRuleset, TerritorySeason, TerritorySite,
    TerritoryStructureRule,
};
