//! Environment-driven configuration for the DNS-over-HTTPS resolver.

use std::{
    env,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
};

use hickory_proto::rr::Name;

const DEFAULT_BIND_ADDR: &str = "0.0.0.0:8053";

/// Runtime configuration loaded from environment variables.
#[derive(Debug, PartialEq, Eq)]
pub struct Config {
    /// Address on which the HTTP server listens.
    pub bind_addr: SocketAddr,
    /// The only hostname for which the resolver returns relay addresses.
    pub target_hostname: String,
    /// IPv4 address returned for the target hostname.
    pub relay_ipv4: Ipv4Addr,
    /// Optional IPv6 address returned for the target hostname.
    pub relay_ipv6: Option<Ipv6Addr>,
}

/// Errors returned when resolver configuration is missing or invalid.
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
    /// Returns [`ConfigError`] when `TARGET_HOSTNAME` or `RELAY_IPV4` is absent,
    /// or when a configured hostname or address is invalid.
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
        let target_hostname = parse_hostname(
            "TARGET_HOSTNAME",
            lookup("TARGET_HOSTNAME").ok_or(ConfigError::Missing { key: "TARGET_HOSTNAME" })?,
        )?;
        let relay_ipv4 = parse(
            "RELAY_IPV4",
            lookup("RELAY_IPV4").ok_or(ConfigError::Missing { key: "RELAY_IPV4" })?,
        )?;
        let relay_ipv6 =
            lookup("RELAY_IPV6").map(|value| parse("RELAY_IPV6", value)).transpose()?;

        Ok(Self { bind_addr, target_hostname, relay_ipv4, relay_ipv6 })
    }
}

fn parse<T>(key: &'static str, value: String) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
{
    value.parse().map_err(|_error| ConfigError::Invalid { key, value })
}

fn parse_hostname(key: &'static str, value: String) -> Result<String, ConfigError> {
    match Name::from_ascii(&value) {
        Ok(name) if name.num_labels() > 0 => {
            Ok(name.to_ascii().trim_end_matches('.').to_ascii_lowercase())
        }
        Ok(_) | Err(_) => Err(ConfigError::Invalid { key, value }),
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
    fn optional_values_should_use_development_defaults() {
        let config = Config::from_lookup(lookup(HashMap::from([
            ("TARGET_HOSTNAME", "example.com"),
            ("RELAY_IPV4", "203.0.113.10"),
        ])))
        .expect("configuration should be valid");

        assert_eq!(
            config,
            Config {
                bind_addr: "0.0.0.0:8053".parse().expect("fixture should be valid"),
                target_hostname: "example.com".to_string(),
                relay_ipv4: Ipv4Addr::new(203, 0, 113, 10),
                relay_ipv6: None,
            }
        );
    }

    #[test]
    fn target_hostname_should_be_required() {
        let error = Config::from_lookup(lookup(HashMap::from([("RELAY_IPV4", "203.0.113.10")])))
            .expect_err("target hostname should be absent");

        assert_eq!(error, ConfigError::Missing { key: "TARGET_HOSTNAME" });
    }

    #[test]
    fn relay_ipv4_should_be_required() {
        let error =
            Config::from_lookup(lookup(HashMap::from([("TARGET_HOSTNAME", "example.com")])))
                .expect_err("relay address should be absent");

        assert_eq!(error, ConfigError::Missing { key: "RELAY_IPV4" });
    }

    #[test]
    fn target_hostname_should_be_normalized() {
        let config = Config::from_lookup(lookup(HashMap::from([
            ("TARGET_HOSTNAME", "ExAmPlE.CoM."),
            ("RELAY_IPV4", "203.0.113.10"),
        ])))
        .expect("configuration should be valid");

        assert_eq!(config.target_hostname, "example.com");
    }

    #[test]
    fn invalid_target_hostname_should_be_rejected() {
        let error = Config::from_lookup(lookup(HashMap::from([
            ("TARGET_HOSTNAME", "not a hostname"),
            ("RELAY_IPV4", "203.0.113.10"),
        ])))
        .expect_err("target hostname should be invalid");

        assert_eq!(
            error,
            ConfigError::Invalid { key: "TARGET_HOSTNAME", value: "not a hostname".to_string() }
        );
    }

    #[test]
    fn empty_target_hostname_should_be_rejected() {
        let error = Config::from_lookup(lookup(HashMap::from([
            ("TARGET_HOSTNAME", ""),
            ("RELAY_IPV4", "203.0.113.10"),
        ])))
        .expect_err("target hostname should be empty");

        assert_eq!(error, ConfigError::Invalid { key: "TARGET_HOSTNAME", value: String::new() });
    }

    #[test]
    fn root_target_hostname_should_be_rejected() {
        let error = Config::from_lookup(lookup(HashMap::from([
            ("TARGET_HOSTNAME", "."),
            ("RELAY_IPV4", "203.0.113.10"),
        ])))
        .expect_err("target hostname should be the DNS root");

        assert_eq!(error, ConfigError::Invalid { key: "TARGET_HOSTNAME", value: ".".to_string() });
    }

    #[test]
    fn invalid_bind_addr_should_be_rejected() {
        let error = Config::from_lookup(lookup(HashMap::from([
            ("TARGET_HOSTNAME", "example.com"),
            ("RELAY_IPV4", "203.0.113.10"),
            ("BIND_ADDR", "localhost"),
        ])))
        .expect_err("bind address should be invalid");

        assert_eq!(
            error,
            ConfigError::Invalid { key: "BIND_ADDR", value: "localhost".to_string() }
        );
    }

    #[test]
    fn invalid_relay_ipv4_should_be_rejected() {
        let error = Config::from_lookup(lookup(HashMap::from([
            ("TARGET_HOSTNAME", "example.com"),
            ("RELAY_IPV4", "2001:db8::1"),
        ])))
        .expect_err("relay address should not be IPv4");

        assert_eq!(
            error,
            ConfigError::Invalid { key: "RELAY_IPV4", value: "2001:db8::1".to_string() }
        );
    }

    #[test]
    fn configured_relay_ipv6_should_be_loaded() {
        let config = Config::from_lookup(lookup(HashMap::from([
            ("TARGET_HOSTNAME", "example.com"),
            ("RELAY_IPV4", "203.0.113.10"),
            ("RELAY_IPV6", "2001:db8::10"),
        ])))
        .expect("configuration should be valid");

        assert_eq!(config.relay_ipv6, Some(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0x10)));
    }

    #[test]
    fn invalid_relay_ipv6_should_be_rejected() {
        let error = Config::from_lookup(lookup(HashMap::from([
            ("TARGET_HOSTNAME", "example.com"),
            ("RELAY_IPV4", "203.0.113.10"),
            ("RELAY_IPV6", "203.0.113.11"),
        ])))
        .expect_err("relay address should not be IPv4");

        assert_eq!(
            error,
            ConfigError::Invalid { key: "RELAY_IPV6", value: "203.0.113.11".to_string() }
        );
    }
}
