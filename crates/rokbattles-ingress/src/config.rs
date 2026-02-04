//! Environment-driven configuration for the ingress service.

use std::env;
use std::num::NonZeroU32;

/// Runtime configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: String,
    pub mongo_uri: String,
    pub clamav_enabled: bool,
    pub clamav_addr: String,
    pub clamav_timeout_ms: u64,
    pub zstd_level: i32,
    pub max_upload_bytes: usize,
    pub rate_limit_per_minute: NonZeroU32,
    pub rate_limit_burst: NonZeroU32,
    pub rate_limit_key: RateLimitKey,
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
        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8000".to_string());
        let mongo_uri = required_env("MONGODB_URI")?;
        let clamav_enabled = parse_bool(env::var("CLAMAV_ENABLED").ok(), false)?;
        let clamav_addr = env::var("CLAMAV_ADDR").unwrap_or_else(|_| "127.0.0.1:3310".to_string());
        let clamav_timeout_ms = parse_u64(
            "CLAMAV_TIMEOUT_MS",
            env::var("CLAMAV_TIMEOUT_MS").ok(),
            15_000,
        )?;
        let zstd_level = parse_i32("ZSTD_LEVEL", env::var("ZSTD_LEVEL").ok(), 3)?;
        let max_upload_bytes = parse_usize(
            "MAX_UPLOAD_BYTES",
            env::var("MAX_UPLOAD_BYTES").ok(),
            25 * 1024 * 1024,
        )?;
        let rate_limit_per_minute = parse_nonzero_u32(
            "RATE_LIMIT_PER_MINUTE",
            env::var("RATE_LIMIT_PER_MINUTE").ok(),
            765,
        )?;
        let rate_limit_burst =
            parse_nonzero_u32("RATE_LIMIT_BURST", env::var("RATE_LIMIT_BURST").ok(), 1530)?;
        let rate_limit_key =
            parse_rate_limit_key(env::var("RATE_LIMIT_KEY").ok(), RateLimitKey::Peer)?;

        Ok(Self {
            bind_addr,
            mongo_uri,
            clamav_enabled,
            clamav_addr,
            clamav_timeout_ms,
            zstd_level,
            max_upload_bytes,
            rate_limit_per_minute,
            rate_limit_burst,
            rate_limit_key,
        })
    }
}

/// Rate limit key strategy used by the governor middleware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitKey {
    /// Use the peer IP from the TCP connection.
    Peer,
    /// Use Cloudflare's `CF-Connecting-IP` header (fallbacks to peer IP when missing).
    Cloudflare,
}

fn required_env(key: &'static str) -> Result<String, ConfigError> {
    env::var(key).map_err(|_| ConfigError::Missing { key })
}

fn parse_bool(value: Option<String>, default: bool) -> Result<bool, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(ConfigError::Invalid {
            key: "CLAMAV_ENABLED",
            value,
        }),
    }
}

fn parse_u64(key: &'static str, value: Option<String>, default: u64) -> Result<u64, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    value
        .parse::<u64>()
        .map_err(|_| ConfigError::Invalid { key, value })
}

fn parse_i32(key: &'static str, value: Option<String>, default: i32) -> Result<i32, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    value
        .parse::<i32>()
        .map_err(|_| ConfigError::Invalid { key, value })
}

fn parse_usize(
    key: &'static str,
    value: Option<String>,
    default: usize,
) -> Result<usize, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    value
        .parse::<usize>()
        .map_err(|_| ConfigError::Invalid { key, value })
}

fn parse_nonzero_u32(
    key: &'static str,
    value: Option<String>,
    default: u32,
) -> Result<NonZeroU32, ConfigError> {
    let parsed = match value {
        Some(value) => value
            .parse::<u32>()
            .map_err(|_| ConfigError::Invalid { key, value })?,
        None => default,
    };
    NonZeroU32::new(parsed).ok_or_else(|| ConfigError::Invalid {
        key,
        value: parsed.to_string(),
    })
}

fn parse_rate_limit_key(
    value: Option<String>,
    default: RateLimitKey,
) -> Result<RateLimitKey, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    match value.to_ascii_lowercase().as_str() {
        "peer" => Ok(RateLimitKey::Peer),
        "cloudflare" | "cf" => Ok(RateLimitKey::Cloudflare),
        _ => Err(ConfigError::Invalid {
            key: "RATE_LIMIT_KEY",
            value,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rate_limit_key() {
        assert_eq!(
            parse_rate_limit_key(Some("peer".to_string()), RateLimitKey::Peer).unwrap(),
            RateLimitKey::Peer
        );
        assert_eq!(
            parse_rate_limit_key(Some("cloudflare".to_string()), RateLimitKey::Peer).unwrap(),
            RateLimitKey::Cloudflare
        );
        assert!(parse_rate_limit_key(Some("nope".to_string()), RateLimitKey::Peer).is_err());
    }
}
