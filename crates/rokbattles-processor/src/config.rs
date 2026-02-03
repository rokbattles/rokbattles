//! Environment-driven configuration for the processor.

use std::env;
use std::time::Duration;

/// Runtime configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    pub mongo_uri: String,
    pub batch_size: i64,
    pub concurrency: usize,
    pub idle_sleep: Duration,
}

/// Errors returned when configuration is missing or invalid.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required env var: {key}")]
    Missing { key: &'static str },
    #[error("invalid value for {key}: {value}")]
    Invalid { key: &'static str, value: String },
}

impl Config {
    /// Load configuration from the environment (and `.env` if present).
    pub fn from_env() -> Result<Self, ConfigError> {
        let mongo_uri = required_env("MONGODB_URI")?;
        let batch_size = parse_i64(
            "PROCESSOR_BATCH_SIZE",
            env::var("PROCESSOR_BATCH_SIZE").ok(),
            500,
        )?;
        let concurrency = parse_usize(
            "PROCESSOR_CONCURRENCY",
            env::var("PROCESSOR_CONCURRENCY").ok(),
            8,
        )?;
        let idle_sleep = parse_duration_secs(
            "PROCESSOR_IDLE_SLEEP_SECS",
            env::var("PROCESSOR_IDLE_SLEEP_SECS").ok(),
            15,
        )?;

        Ok(Self {
            mongo_uri,
            batch_size,
            concurrency,
            idle_sleep,
        })
    }
}

fn required_env(key: &'static str) -> Result<String, ConfigError> {
    env::var(key).map_err(|_| ConfigError::Missing { key })
}

fn parse_i64(key: &'static str, value: Option<String>, default: i64) -> Result<i64, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    let parsed = value
        .parse::<i64>()
        .map_err(|_| ConfigError::Invalid { key, value })?;
    if parsed <= 0 {
        return Err(ConfigError::Invalid {
            key,
            value: parsed.to_string(),
        });
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
    let parsed = value
        .parse::<usize>()
        .map_err(|_| ConfigError::Invalid { key, value })?;
    if parsed == 0 {
        return Err(ConfigError::Invalid {
            key,
            value: parsed.to_string(),
        });
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
    let parsed = value
        .parse::<u64>()
        .map_err(|_| ConfigError::Invalid { key, value })?;
    if parsed == 0 {
        return Err(ConfigError::Invalid {
            key,
            value: parsed.to_string(),
        });
    }
    Ok(Duration::from_secs(parsed))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn parse_duration_secs_uses_default() {
        let duration = parse_duration_secs("TEST", None, 5).unwrap();
        assert_eq!(duration, Duration::from_secs(5));
    }

    #[test]
    fn parse_duration_secs_rejects_zero() {
        assert!(parse_duration_secs("TEST", Some("0".into()), 1).is_err());
    }
}
