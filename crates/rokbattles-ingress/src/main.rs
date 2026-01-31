#![forbid(unsafe_code)]

mod clamav;
mod config;
mod error;
mod handlers;
mod rate_limit;
mod state;
mod storage;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use mongodb::options::ClientOptions;
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;
use tracing::info;

use crate::config::Config;
use crate::rate_limit::RateLimitKeyExtractor;
use crate::state::AppState;
use crate::storage::Storage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "rokbattles_ingress=info,axum=info".into());
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let client_options = ClientOptions::parse(&config.mongo_uri).await?;
    let db_name = client_options.default_database.clone().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "MongoDB URI must include a default database",
        )
    })?;
    let client = mongodb::Client::with_options(client_options)?;
    let db = client.database(&db_name);

    let storage = Storage::new(db);
    storage.ensure_indexes().await?;

    let state = Arc::new(AppState { config, storage });

    let rate_limit_layer = {
        let per_minute = state.config.rate_limit_per_minute.get();
        let burst_size = state.config.rate_limit_burst.get();
        let period_ms = (60_000u64 / u64::from(per_minute)).max(1);

        let mut builder = GovernorConfigBuilder::default()
            .key_extractor(RateLimitKeyExtractor::new(state.config.rate_limit_key));
        builder.per_millisecond(period_ms).burst_size(burst_size);
        let mut builder = builder.use_headers();
        let governor_config = builder.finish().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid rate limit config",
            )
        })?;
        GovernorLayer::new(governor_config)
    };

    let app = Router::new()
        .route("/health", get(handlers::health))
        .route("/v2/upload", post(handlers::upload).layer(rate_limit_layer))
        .with_state(state.clone())
        .layer(DefaultBodyLimit::max(state.config.max_upload_bytes));

    info!("listening on {}", state.config.bind_addr);
    let listener = tokio::net::TcpListener::bind(&state.config.bind_addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
