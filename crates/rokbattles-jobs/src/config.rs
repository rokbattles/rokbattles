//! Configuration loaded from environment variables.

use std::env;

/// Settings the jobs process needs at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub mongo_uri: String,
    pub sentry_dsn: Option<String>,
}

/// Configuration errors caught before the jobs start.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing required env var: {key}")]
    Missing { key: &'static str },
}

impl Config {
    /// Read configuration from the current environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| env::var(key).ok())
    }

    fn from_lookup<F>(lookup: F) -> Result<Self, ConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let mongo_uri = lookup("MONGODB_URI").ok_or(ConfigError::Missing { key: "MONGODB_URI" })?;
        let sentry_dsn = lookup("SENTRY_DSN").filter(|value| !value.is_empty());

        Ok(Self { mongo_uri, sentry_dsn })
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
    fn loads_required_values() {
        let cfg = Config::from_lookup(lookup(HashMap::from([(
            "MONGODB_URI",
            "mongodb://localhost:27017/rokbattles",
        )])))
        .expect("config");

        assert_eq!(cfg.mongo_uri, "mongodb://localhost:27017/rokbattles");
    }

    #[test]
    fn loads_optional_sentry_dsn() {
        let cfg = Config::from_lookup(lookup(HashMap::from([
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
            ("SENTRY_DSN", "https://example@sentry.io/123"),
        ])))
        .expect("config");

        assert_eq!(cfg.sentry_dsn, Some("https://example@sentry.io/123".to_string()));
    }

    #[test]
    fn requires_mongo_uri() {
        let err = Config::from_lookup(lookup(HashMap::new())).expect_err("missing uri");
        assert_eq!(err, ConfigError::Missing { key: "MONGODB_URI" });
    }
}
