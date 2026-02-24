//! Environment-driven configuration for the API service.

use std::env;

/// Runtime configuration loaded from environment variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub bind_addr: String,
    pub mongo_uri: String,
    pub log_filter: String,
}

/// Errors returned when required configuration is missing.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing required env var: MONGODB_URI")]
    MissingMongoUri,
}

impl Config {
    /// Load configuration from environment (and `.env` if present).
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| env::var(key).ok())
    }

    fn from_lookup<F>(lookup: F) -> Result<Self, ConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let bind_addr = lookup("BIND_ADDR").unwrap_or_else(|| "0.0.0.0:8001".to_string());
        let mongo_uri = lookup("MONGODB_URI").ok_or(ConfigError::MissingMongoUri)?;
        let log_filter =
            lookup("RUST_LOG").unwrap_or_else(|| "rokbattles_api=info,axum=info".to_string());

        Ok(Self {
            bind_addr,
            mongo_uri,
            log_filter,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn lookup(vars: HashMap<&'static str, &'static str>) -> impl Fn(&str) -> Option<String> {
        move |key| vars.get(key).map(|value| (*value).to_string())
    }

    #[test]
    fn uses_defaults_for_optional_values() {
        let cfg = Config::from_lookup(lookup(HashMap::from([(
            "MONGODB_URI",
            "mongodb://localhost:27017/rokbattles",
        )])))
        .expect("config");

        assert_eq!(cfg.bind_addr, "0.0.0.0:8001");
        assert_eq!(cfg.log_filter, "rokbattles_api=info,axum=info");
        assert_eq!(cfg.mongo_uri, "mongodb://localhost:27017/rokbattles");
    }

    #[test]
    fn requires_mongo_uri() {
        let err = Config::from_lookup(lookup(HashMap::new())).expect_err("missing uri");
        assert_eq!(err, ConfigError::MissingMongoUri);
    }
}
