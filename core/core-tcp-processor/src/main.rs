#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Background worker that decrypts captured TCP streams and stores decoded packets.

mod api_map;
mod artifact;
mod config;
mod descriptor;
mod error;
mod processing;
mod proto;
mod storage;
mod stream;

use std::path::PathBuf;

use mongodb::options::ClientOptions;
use tracing::debug;

use crate::{config::Config, error::ProcessorError, processing::process_loop, storage::Storage};

#[tokio::main]
async fn main() -> Result<(), ProcessorError> {
    let dotenv_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env");
    let _dotenv_result = dotenvy::from_path(&dotenv_path);

    let config = Config::from_env()?;
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("{}=info", env!("CARGO_CRATE_NAME")).into());
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let client_options = ClientOptions::parse(&config.mongo_uri).await?;
    let db_name = client_options.default_database.clone().ok_or(ProcessorError::MissingDatabase)?;
    let client = mongodb::Client::with_options(client_options)?;
    let db = client.database(&db_name);
    debug!(database = %db.name(), "connected to MongoDB");

    let storage = Storage::new(db);
    storage.ensure_indexes().await?;

    process_loop(storage, config).await
}
