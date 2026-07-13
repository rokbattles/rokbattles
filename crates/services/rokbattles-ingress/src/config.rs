//! Environment-driven configuration for the ingress service.

use std::env;

/// Runtime configuration loaded from environment variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub bind_addr: String,
    pub mongo_uri: String,
    pub sentry_dsn: Option<String>,
    pub clamav_enabled: bool,
    pub clamav_addr: String,
    pub clamav_timeout_ms: u64,
    pub raw_zstd_level: i32,
    pub max_upload_bytes: usize,
}

/// Errors returned when configuration is missing or invalid.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing required env var: {key}")]
    Missing { key: &'static str },
    #[error("invalid value for {key}: {value}")]
    Invalid { key: &'static str, value: String },
}

impl Config {
    /// Load configuration from the environment (and `.env` if present).
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| env::var(key).ok())
    }

    fn from_lookup<F>(lookup: F) -> Result<Self, ConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let bind_addr = lookup("BIND_ADDR").unwrap_or_else(|| "0.0.0.0:8000".to_string());
        let mongo_uri = required(&lookup, "MONGODB_URI")?;
        let sentry_dsn = lookup("SENTRY_DSN").filter(|value| !value.is_empty());
        let clamav_enabled = parse_bool("CLAMAV_ENABLED", lookup("CLAMAV_ENABLED"), false)?;
        let clamav_addr = lookup("CLAMAV_ADDR").unwrap_or_else(|| "127.0.0.1:3310".to_string());
        let clamav_timeout_ms =
            parse_u64("CLAMAV_TIMEOUT_MS", lookup("CLAMAV_TIMEOUT_MS"), 15_000)?;
        let raw_zstd_level = parse_i32("RAW_ZSTD_LEVEL", lookup("RAW_ZSTD_LEVEL"), 6)?;
        let max_upload_bytes =
            parse_usize("MAX_UPLOAD_BYTES", lookup("MAX_UPLOAD_BYTES"), 25 * 1024 * 1024)?;

        Ok(Self {
            bind_addr,
            mongo_uri,
            sentry_dsn,
            clamav_enabled,
            clamav_addr,
            clamav_timeout_ms,
            raw_zstd_level,
            max_upload_bytes,
        })
    }
}

fn required<F>(lookup: &F, key: &'static str) -> Result<String, ConfigError>
where
    F: Fn(&str) -> Option<String>,
{
    lookup(key).ok_or(ConfigError::Missing { key })
}

fn parse_bool(
    key: &'static str,
    value: Option<String>,
    default: bool,
) -> Result<bool, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(ConfigError::Invalid { key, value }),
    }
}

fn parse_u64(key: &'static str, value: Option<String>, default: u64) -> Result<u64, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    value.parse::<u64>().map_err(|_| ConfigError::Invalid { key, value })
}

fn parse_i32(key: &'static str, value: Option<String>, default: i32) -> Result<i32, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    value.parse::<i32>().map_err(|_| ConfigError::Invalid { key, value })
}

fn parse_usize(
    key: &'static str,
    value: Option<String>,
    default: usize,
) -> Result<usize, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    value.parse::<usize>().map_err(|_| ConfigError::Invalid { key, value })
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

        assert_eq!(
            cfg,
            Config {
                bind_addr: "0.0.0.0:8000".to_string(),
                mongo_uri: "mongodb://localhost:27017/rokbattles".to_string(),
                sentry_dsn: None,
                clamav_enabled: false,
                clamav_addr: "127.0.0.1:3310".to_string(),
                clamav_timeout_ms: 15_000,
                raw_zstd_level: 6,
                max_upload_bytes: 25 * 1024 * 1024,
            }
        );
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

    #[test]
    fn loads_raw_zstd_level() {
        let cfg = Config::from_lookup(lookup(HashMap::from([
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
            ("RAW_ZSTD_LEVEL", "8"),
        ])))
        .expect("config");

        assert_eq!(cfg.raw_zstd_level, 8);
    }
}
