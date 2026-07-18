//! Environment-driven configuration for the processor.

use std::{env, time::Duration};

/// Runtime configuration loaded from environment variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub mongo_uri: String,
    pub sentry_dsn: Option<String>,
    pub batch_size: i64,
    pub concurrency: usize,
    pub idle_sleep: Duration,
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
        let mongo_uri = required(&lookup, "MONGODB_URI")?;
        let sentry_dsn = lookup("SENTRY_DSN").filter(|value| !value.is_empty());
        let batch_size = parse_i64("PROCESSOR_BATCH_SIZE", lookup("PROCESSOR_BATCH_SIZE"), 500)?;
        let concurrency = parse_usize("PROCESSOR_CONCURRENCY", lookup("PROCESSOR_CONCURRENCY"), 8)?;
        let idle_sleep = parse_duration_secs(
            "PROCESSOR_IDLE_SLEEP_SECS",
            lookup("PROCESSOR_IDLE_SLEEP_SECS"),
            15,
        )?;
        Ok(Self { mongo_uri, sentry_dsn, batch_size, concurrency, idle_sleep })
    }
}

fn required<F>(lookup: &F, key: &'static str) -> Result<String, ConfigError>
where
    F: Fn(&str) -> Option<String>,
{
    lookup(key).ok_or(ConfigError::Missing { key })
}

fn parse_i64(key: &'static str, value: Option<String>, default: i64) -> Result<i64, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    let parsed = value.parse::<i64>().map_err(|_| ConfigError::Invalid { key, value })?;
    if parsed <= 0 {
        return Err(ConfigError::Invalid { key, value: parsed.to_string() });
    }
    Ok(parsed)
}

fn parse_usize(
    key: &'static str,
    value: Option<String>,
    default: usize,
) -> Result<usize, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    let parsed = value.parse::<usize>().map_err(|_| ConfigError::Invalid { key, value })?;
    if parsed == 0 {
        return Err(ConfigError::Invalid { key, value: parsed.to_string() });
    }
    Ok(parsed)
}

fn parse_duration_secs(
    key: &'static str,
    value: Option<String>,
    default_secs: u64,
) -> Result<Duration, ConfigError> {
    let Some(value) = value else {
        return Ok(Duration::from_secs(default_secs));
    };
    let parsed = value.parse::<u64>().map_err(|_| ConfigError::Invalid { key, value })?;
    if parsed == 0 {
        return Err(ConfigError::Invalid { key, value: parsed.to_string() });
    }
    Ok(Duration::from_secs(parsed))
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
                mongo_uri: "mongodb://localhost:27017/rokbattles".to_string(),
                sentry_dsn: None,
                batch_size: 500,
                concurrency: 8,
                idle_sleep: Duration::from_secs(15),
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
    fn parse_i64_uses_default() {
        let value = parse_i64("TEST", None, 42).unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn parse_i64_rejects_zero() {
        assert!(parse_i64("TEST", Some("0".into()), 42).is_err());
    }

    #[test]
    fn parse_usize_rejects_zero() {
        assert!(parse_usize("TEST", Some("0".into()), 3).is_err());
    }

    #[test]
    fn loads_processing_settings() {
        let cfg = Config::from_lookup(lookup(HashMap::from([
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
            ("PROCESSOR_BATCH_SIZE", "25"),
            ("PROCESSOR_CONCURRENCY", "4"),
            ("PROCESSOR_IDLE_SLEEP_SECS", "2"),
        ])))
        .expect("config");
        assert_eq!(cfg.batch_size, 25);
        assert_eq!(cfg.concurrency, 4);
        assert_eq!(cfg.idle_sleep, Duration::from_secs(2));
    }

    #[test]
    fn numeric_settings_reject_invalid_values() {
        assert!(parse_i64("I64", Some("bad".into()), 1).is_err());
        assert!(parse_i64("I64", Some("-1".into()), 1).is_err());
        assert!(parse_usize("USIZE", Some("bad".into()), 1).is_err());
        assert!(parse_duration_secs("DURATION", Some("bad".into()), 1).is_err());
    }

    #[test]
    fn parse_duration_secs_uses_default() {
        let duration = parse_duration_secs("TEST", None, 5).unwrap();
        assert_eq!(duration, Duration::from_secs(5));
    }

    #[test]
    fn parse_duration_secs_rejects_zero() {
        assert!(parse_duration_secs("TEST", Some("0".into()), 1).is_err());
    }
}
