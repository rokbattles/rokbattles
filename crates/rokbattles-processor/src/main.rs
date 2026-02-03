#![forbid(unsafe_code)]

//! Background processor for raw mail documents.

mod config;
mod error;
mod mail;
mod processing;
mod storage;

use mongodb::options::ClientOptions;
use tracing::debug;

use crate::config::Config;
use crate::error::ProcessorError;
use crate::processing::process_loop;
use crate::storage::Storage;

#[tokio::main]
async fn main() -> Result<(), ProcessorError> {
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("{}=info", env!("CARGO_CRATE_NAME")).into());
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let client_options = ClientOptions::parse(&config.mongo_uri).await?;
    let db_name = client_options
        .default_database
        .clone()
        .ok_or(ProcessorError::MissingDatabase)?;
    let client = mongodb::Client::with_options(client_options)?;
    let db = client.database(&db_name);
    debug!(database = %db.name(), "connected to MongoDB");

    let storage = Storage::new(db);
    storage.ensure_indexes().await?;

    process_loop(storage, config).await
}
