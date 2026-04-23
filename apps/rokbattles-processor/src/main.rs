#![forbid(unsafe_code)]

//! Background processor for raw mail documents.

mod config;
mod error;
mod mail;
mod processing;
mod storage;

use mongodb::options::ClientOptions;
use tracing::debug;

use crate::{config::Config, error::ProcessorError, processing::process_loop, storage::Storage};

#[tokio::main]
async fn main() -> Result<(), ProcessorError> {
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("{}=info", env!("CARGO_CRATE_NAME")).into());
    tracing_subscriber::fmt().with_env_filter(filter).init();

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
    let db_name = client_options.default_database.clone().ok_or(ProcessorError::MissingDatabase)?;
    let client = mongodb::Client::with_options(client_options)?;
    let db = client.database(&db_name);
    debug!(database = %db.name(), "connected to MongoDB");

    let storage = Storage::new(db);
    storage.ensure_indexes().await?;

    process_loop(storage, config).await
}
