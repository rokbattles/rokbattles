//! Errors used by the jobs process.

use crate::config::ConfigError;

/// Top-level error type for the jobs binary.
#[derive(Debug, thiserror::Error)]
pub enum JobsError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Mongo(#[from] mongodb::error::Error),
    #[error(transparent)]
    Scheduler(#[from] tokio_cron_scheduler::JobSchedulerError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    BsonSerialization(#[from] mongodb::bson::ser::Error),
    #[error(transparent)]
    Yaml(#[from] yaml_serde::Error),
    #[error("MONGODB_URI must include a default database")]
    MissingDatabase,
    #[error("Commander dataset contains no legendary commanders")]
    MissingLegendaryCommanders,
    #[error("invalid Combat Lab data: {0}")]
    InvalidCombatLabData(String),
    #[error("invalid materialized DRASTC data: {0}")]
    InvalidDrastcData(String),
}
