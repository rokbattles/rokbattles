#![forbid(unsafe_code)]

#[cfg(all(target_os = "linux", target_env = "musl"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod clamav;
mod config;
mod error;
mod handlers;
mod mail_update;
mod raw_mail;
mod state;
mod storage;

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use mongodb::options::ClientOptions;
use rokbattles_mail_reconstructor::MailReconstructor;
use tracing::info;

use crate::{config::Config, state::AppState, storage::Storage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dotenv_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env");
    dotenvy::from_path(&dotenv_path).ok();

    let config = Config::from_env()?;
    let mail_reconstructor = Arc::new(MailReconstructor::load("artifacts/artifacts.json")?);
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

    let state = Arc::new(AppState { config, storage, mail_reconstructor });

    let app = Router::new()
        .route("/health", get(handlers::health))
        .route("/v2/upload", post(handlers::upload))
        .route("/v2/relay/upload", post(handlers::upload_relay))
        .route("/v2/tcp-stream", post(handlers::upload_tcp_stream))
        .with_state(state.clone())
        .layer(DefaultBodyLimit::max(state.config.max_upload_bytes));

    info!("listening on {}", state.config.bind_addr);
    let listener = tokio::net::TcpListener::bind(&state.config.bind_addr).await?;
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;

    Ok(())
}
