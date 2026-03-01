#![forbid(unsafe_code)]

use std::sync::Arc;

use mongodb::options::ClientOptions;
use rokbattles_api::build_router;
use rokbattles_api::config::Config;
use rokbattles_api::db::{MongoAuthStore, ReportsStore};
use rokbattles_api::state::{AppState, DiscordOAuthConfig};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;
    tracing_subscriber::fmt()
        .with_env_filter(config.log_filter.clone())
        .init();

    let client_options = ClientOptions::parse(&config.mongo_uri).await?;
    let database_name = client_options.default_database.clone().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "MONGODB_URI must include a default database",
        )
    })?;

    let client = mongodb::Client::with_options(client_options)?;
    let database = client.database(&database_name);
    let reports_store = ReportsStore::new(database.clone());
    reports_store.ensure_indexes().await?;

    let auth_store = Arc::new(MongoAuthStore::new(database));
    auth_store.ensure_indexes().await?;

    let discord_oauth = DiscordOAuthConfig {
        client_id: config.discord_client_id.clone(),
        client_secret: config.discord_client_secret.clone(),
        redirect_uri: config.discord_redirect_uri.clone(),
    };
    let state = Arc::new(AppState::new(
        auth_store,
        reports_store,
        config.cron_secret.clone(),
        discord_oauth,
    ));
    let app = build_router(state);

    info!(bind_addr = %config.bind_addr, "starting rokbattles-api");
    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
