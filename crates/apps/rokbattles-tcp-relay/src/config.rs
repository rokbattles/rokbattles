//! Environment-driven configuration for the TCP relay.

use std::{env, net::SocketAddr};

const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3101";

/// Runtime configuration loaded from environment variables.
#[derive(Debug, PartialEq, Eq)]
pub struct Config {
    /// Address on which the relay accepts client connections.
    pub bind_addr: SocketAddr,
    /// Host and port to which each client connection is forwarded.
    pub upstream_addr: String,
    /// Bearer token shared with ingress.
    pub relay_token: String,
}

/// Errors returned when relay configuration is missing or invalid.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    /// A required environment variable was not set.
    #[error("missing required env var: {key}")]
    Missing { key: &'static str },
    /// An environment variable could not be parsed.
    #[error("invalid value for {key}: {value}")]
    Invalid { key: &'static str, value: String },
}

impl Config {
    /// Load configuration from the process environment.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] when a required value is absent or a configured
    /// address is invalid.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| env::var(key).ok())
    }

    fn from_lookup<F>(lookup: F) -> Result<Self, ConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let bind_addr = parse(
            "BIND_ADDR",
            lookup("BIND_ADDR").unwrap_or_else(|| DEFAULT_BIND_ADDR.to_string()),
        )?;
        let upstream_addr = validate_upstream_addr(
            lookup("UPSTREAM_ADDR").ok_or(ConfigError::Missing { key: "UPSTREAM_ADDR" })?,
        )?;
        let relay_token = lookup("RELAY_TOKEN")
            .filter(|value| !value.is_empty())
            .ok_or(ConfigError::Missing { key: "RELAY_TOKEN" })?;
        Ok(Self { bind_addr, upstream_addr, relay_token })
    }
}

fn parse<T>(key: &'static str, value: String) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
{
    value.parse().map_err(|_error| ConfigError::Invalid { key, value })
}

fn validate_upstream_addr(value: String) -> Result<String, ConfigError> {
    let valid = value
        .parse::<SocketAddr>()
        .map_or_else(|_| valid_hostname_addr(&value), |address| address.port() != 0);

    if valid { Ok(value) } else { Err(ConfigError::Invalid { key: "UPSTREAM_ADDR", value }) }
}

fn valid_hostname_addr(value: &str) -> bool {
    let Some((hostname, port)) = value.rsplit_once(':') else {
        return false;
    };
    let hostname = hostname.strip_suffix('.').unwrap_or(hostname);
    valid_hostname(hostname) && port.parse::<u16>().is_ok_and(|port| port != 0)
}

fn valid_hostname(hostname: &str) -> bool {
    !hostname.is_empty()
        && hostname.len() <= 253
        && hostname.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && label.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
                && label.as_bytes().last().is_some_and(u8::is_ascii_alphanumeric)
        })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn lookup(vars: HashMap<&'static str, &'static str>) -> impl Fn(&str) -> Option<String> {
        move |key| vars.get(key).map(|value| (*value).to_string())
    }

    #[test]
    fn optional_values_should_use_defaults() {
        let config = Config::from_lookup(lookup(HashMap::from([
            ("UPSTREAM_ADDR", "example.com:3101"),
            ("RELAY_TOKEN", "secret"),
        ])))
        .expect("configuration should be valid");

        assert_eq!(
            config,
            Config {
                bind_addr: "0.0.0.0:3101".parse().expect("fixture should be valid"),
                upstream_addr: "example.com:3101".to_string(),
                relay_token: "secret".to_string(),
            }
        );
    }

    #[test]
    fn upstream_addr_should_be_required() {
        let error =
            Config::from_lookup(lookup(HashMap::new())).expect_err("upstream should be absent");

        assert_eq!(error, ConfigError::Missing { key: "UPSTREAM_ADDR" });
    }

    #[test]
    fn invalid_bind_addr_should_be_rejected() {
        let error = Config::from_lookup(lookup(HashMap::from([
            ("BIND_ADDR", "localhost"),
            ("UPSTREAM_ADDR", "example.com:3101"),
        ])))
        .expect_err("bind address should be invalid");

        assert_eq!(
            error,
            ConfigError::Invalid { key: "BIND_ADDR", value: "localhost".to_string() }
        );
    }

    #[test]
    fn upstream_addr_without_port_should_be_rejected() {
        let error = Config::from_lookup(lookup(HashMap::from([("UPSTREAM_ADDR", "example.com")])))
            .expect_err("upstream port should be absent");

        assert_eq!(
            error,
            ConfigError::Invalid { key: "UPSTREAM_ADDR", value: "example.com".to_string() }
        );
    }

    #[test]
    fn upstream_addr_with_zero_port_should_be_rejected() {
        let error = Config::from_lookup(lookup(HashMap::from([("UPSTREAM_ADDR", "127.0.0.1:0")])))
            .expect_err("upstream port should be zero");

        assert_eq!(
            error,
            ConfigError::Invalid { key: "UPSTREAM_ADDR", value: "127.0.0.1:0".to_string() }
        );
    }

    #[test]
    fn bracketed_ipv6_upstream_addr_should_be_loaded() {
        let config = Config::from_lookup(lookup(HashMap::from([
            ("UPSTREAM_ADDR", "[2001:db8::10]:3101"),
            ("RELAY_TOKEN", "secret"),
        ])))
        .expect("configuration should be valid");

        assert_eq!(config.upstream_addr, "[2001:db8::10]:3101");
    }

    #[test]
    fn relay_token_should_be_required() {
        let error =
            Config::from_lookup(lookup(HashMap::from([("UPSTREAM_ADDR", "example.com:3101")])))
                .expect_err("relay token should be required");

        assert_eq!(error, ConfigError::Missing { key: "RELAY_TOKEN" });
    }

    #[test]
    fn empty_relay_token_should_be_rejected() {
        let error = Config::from_lookup(lookup(HashMap::from([
            ("UPSTREAM_ADDR", "example.com:3101"),
            ("RELAY_TOKEN", ""),
        ])))
        .expect_err("relay token should not be empty");

        assert_eq!(error, ConfigError::Missing { key: "RELAY_TOKEN" });
    }
}
