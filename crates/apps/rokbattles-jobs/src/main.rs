#![forbid(unsafe_code)]

//! Binary entrypoint for scheduled jobs.

#[cfg(all(target_os = "linux", target_env = "musl"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::{future::Future, io, path::PathBuf, time::Instant};

use mongodb::options::ClientOptions;
use rokbattles_api::db::ReportsStore;
use rokbattles_jobs::{config::Config, error::JobsError, scheduler::build_scheduler};
use tracing::{debug, info};

#[tokio::main]
async fn main() -> Result<(), JobsError> {
    let dotenv_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env");
    // Environment variables can be supplied without a local .env file.
    let _dotenv_result = dotenvy::from_path(&dotenv_path);

    let config = Config::from_env()?;
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(
            |_| format!("{}=info,rokbattles_api=info", env!("CARGO_CRATE_NAME")).into(),
        ))
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

    let shutdown_signal = shutdown_signal()?;
    let client_options = ClientOptions::parse(&config.mongo_uri).await?;
    let db_name = client_options.default_database.clone().ok_or(JobsError::MissingDatabase)?;
    let client = mongodb::Client::with_options(client_options)?;
    let db = client.database(&db_name);
    let indexes_started = Instant::now();
    debug!(database = %db.name(), "ensuring jobs indexes");
    let reports_store = ReportsStore::new(db);
    reports_store.ensure_indexes().await?;
    info!(database = %db_name, elapsed_ms = indexes_started.elapsed().as_millis(), "jobs indexes ready");

    let mut scheduler = build_scheduler(reports_store).await?;
    scheduler.start().await?;
    info!("jobs scheduler started");

    let signal = shutdown_signal.await?;
    info!(signal, "shutting down jobs scheduler and waiting for active jobs");
    let shutdown_started = Instant::now();
    scheduler.shutdown().await?;
    info!(elapsed_ms = shutdown_started.elapsed().as_millis(), "jobs scheduler stopped");

    Ok(())
}

#[cfg(unix)]
fn shutdown_signal() -> io::Result<impl Future<Output = io::Result<&'static str>>> {
    use tokio::signal::unix::{SignalKind, signal};

    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut terminate = signal(SignalKind::terminate())?;
    Ok(async move {
        tokio::select! {
            _ = interrupt.recv() => Ok("SIGINT"),
            _ = terminate.recv() => Ok("SIGTERM"),
        }
    })
}

#[cfg(not(unix))]
fn shutdown_signal() -> io::Result<impl Future<Output = io::Result<&'static str>>> {
    Ok(async {
        tokio::signal::ctrl_c().await?;
        Ok("Ctrl-C")
    })
}
