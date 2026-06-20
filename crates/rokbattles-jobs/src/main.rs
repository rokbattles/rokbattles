#![forbid(unsafe_code)]

//! Binary entrypoint for scheduled jobs.

use std::path::PathBuf;

use mongodb::options::ClientOptions;
use rokbattles_api::db::ReportsStore;
use rokbattles_jobs::{config::Config, error::JobsError, scheduler::build_scheduler};
use tracing::{debug, info};

#[tokio::main]
async fn main() -> Result<(), JobsError> {
    let dotenv_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env");
    dotenvy::from_path(&dotenv_path).ok();

    let config = Config::from_env()?;
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        format!("{}=info,rokbattles_api=info", env!("CARGO_CRATE_NAME")).into()
    });
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
    let db_name = client_options.default_database.clone().ok_or(JobsError::MissingDatabase)?;
    let client = mongodb::Client::with_options(client_options)?;
    let db = client.database(&db_name);
    debug!(database = %db.name(), "connected to MongoDB");

    let reports_store = ReportsStore::new(db);
    reports_store.ensure_indexes().await?;

    let mut scheduler = build_scheduler(reports_store).await?;
    scheduler.start().await?;
    info!("started");

    tokio::signal::ctrl_c().await?;
    info!("shutting down");
    scheduler.shutdown().await?;

    Ok(())
}
