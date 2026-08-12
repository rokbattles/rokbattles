#![forbid(unsafe_code)]

#[cfg(all(target_os = "linux", target_env = "musl"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::{path::PathBuf, sync::Arc};

use mongodb::options::ClientOptions;
use rokbattles_api::{
    build_router,
    config::Config,
    db::{GameLocalizationStore, MongoAuthStore, ReportsStore},
    state::{AppState, DiscordOAuthConfig},
};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dotenv_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env");
    dotenvy::from_path(&dotenv_path).ok();

    let config = Config::from_env()?;
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=info,axum=info", env!("CARGO_CRATE_NAME")).into()),
        )
        .init();

    let _sentry_guard = config.sentry_dsn.as_deref().map(|dsn| {
        sentry::init((
            dsn,
            sentry::ClientOptions {
                release: sentry::release_name!(),
                send_default_pii: false,
                ..Default::default()
            },
        ))
    });

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

    let game_localizations = GameLocalizationStore::new(database.clone());
    game_localizations.ensure_indexes().await?;

    let auth_store = Arc::new(MongoAuthStore::new(database));
    auth_store.ensure_indexes().await?;

    let discord_oauth = DiscordOAuthConfig {
        client_id: config.discord_client_id.clone(),
        client_secret: config.discord_client_secret.clone(),
        redirect_uri: config.discord_redirect_uri.clone(),
    };
    let state =
        Arc::new(AppState::new(auth_store, game_localizations, reports_store, discord_oauth));
    let app = build_router(state);

    info!(bind_addr = %config.bind_addr, "starting rokbattles-api");
    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
