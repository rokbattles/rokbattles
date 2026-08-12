//! Environment-driven configuration for the DNS-over-HTTPS resolver.

use std::{
    env,
    net::{Ipv4Addr, SocketAddr},
};

use hickory_proto::rr::Name;
use reqwest::Url;

const DEFAULT_BIND_ADDR: &str = "0.0.0.0:8053";

/// Runtime configuration loaded from environment variables.
#[derive(Debug, PartialEq, Eq)]
pub struct Config {
    /// Address on which the HTTP server listens.
    pub bind_addr: SocketAddr,
    /// The only hostname for which the resolver returns the relay address.
    pub target_hostname: String,
    /// IPv4 address returned for the target hostname.
    pub relay_ipv4: Ipv4Addr,
    /// DoH resolver used for non-target queries received from Intra.
    pub intra_upstream_doh_url: Url,
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
    /// Returns [`ConfigError`] when `TARGET_HOSTNAME`, `RELAY_IPV4`, or
    /// `INTRA_UPSTREAM_DOH_URL` is absent, or when a configured hostname,
    /// address, or URL is invalid.
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
        let intra_upstream_doh_url = parse_upstream_url(
            "INTRA_UPSTREAM_DOH_URL",
            lookup("INTRA_UPSTREAM_DOH_URL")
                .ok_or(ConfigError::Missing { key: "INTRA_UPSTREAM_DOH_URL" })?,
        )?;

        Ok(Self { bind_addr, target_hostname, relay_ipv4, intra_upstream_doh_url })
    }
}

fn parse<T>(key: &'static str, value: String) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
{
    value.parse().map_err(|_| ConfigError::Invalid { key, value })
}

fn parse_hostname(key: &'static str, value: String) -> Result<String, ConfigError> {
    match Name::from_ascii(&value) {
        Ok(name) if name.num_labels() > 0 => {
            Ok(name.to_ascii().trim_end_matches('.').to_ascii_lowercase())
        }
        Ok(_) | Err(_) => Err(ConfigError::Invalid { key, value }),
    }
}

fn parse_upstream_url(key: &'static str, value: String) -> Result<Url, ConfigError> {
    match Url::parse(&value) {
        Ok(url) if url.scheme() == "https" && url.host_str().is_some() => Ok(url),
        Ok(_) | Err(_) => Err(ConfigError::Invalid { key, value }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    const TEST_UPSTREAM_DOH_URL: &str = "https://dns.example.net/dns-query";

    fn lookup(vars: HashMap<&'static str, &'static str>) -> impl Fn(&str) -> Option<String> {
        move |key| vars.get(key).map(|value| (*value).to_string())
    }

    fn lookup_with_upstream(
        mut vars: HashMap<&'static str, &'static str>,
    ) -> impl Fn(&str) -> Option<String> {
        vars.entry("INTRA_UPSTREAM_DOH_URL").or_insert(TEST_UPSTREAM_DOH_URL);
        lookup(vars)
    }

    #[test]
    fn optional_values_should_use_development_defaults() {
        let config = Config::from_lookup(lookup_with_upstream(HashMap::from([
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
                intra_upstream_doh_url: Url::parse(TEST_UPSTREAM_DOH_URL)
                    .expect("upstream URL fixture should be valid"),
            }
        );
    }

    #[test]
    fn target_hostname_should_be_required() {
        let error = Config::from_lookup(lookup_with_upstream(HashMap::from([(
            "RELAY_IPV4",
            "203.0.113.10",
        )])))
        .expect_err("target hostname should be absent");

        assert_eq!(error, ConfigError::Missing { key: "TARGET_HOSTNAME" });
    }

    #[test]
    fn relay_ipv4_should_be_required() {
        let error = Config::from_lookup(lookup_with_upstream(HashMap::from([(
            "TARGET_HOSTNAME",
            "example.com",
        )])))
        .expect_err("relay address should be absent");

        assert_eq!(error, ConfigError::Missing { key: "RELAY_IPV4" });
    }

    #[test]
    fn target_hostname_should_be_normalized() {
        let config = Config::from_lookup(lookup_with_upstream(HashMap::from([
            ("TARGET_HOSTNAME", "ExAmPlE.CoM."),
            ("RELAY_IPV4", "203.0.113.10"),
        ])))
        .expect("configuration should be valid");

        assert_eq!(config.target_hostname, "example.com");
    }

    #[test]
    fn invalid_target_hostname_should_be_rejected() {
        let error = Config::from_lookup(lookup_with_upstream(HashMap::from([
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
        let error = Config::from_lookup(lookup_with_upstream(HashMap::from([
            ("TARGET_HOSTNAME", ""),
            ("RELAY_IPV4", "203.0.113.10"),
        ])))
        .expect_err("target hostname should be empty");

        assert_eq!(error, ConfigError::Invalid { key: "TARGET_HOSTNAME", value: String::new() });
    }

    #[test]
    fn root_target_hostname_should_be_rejected() {
        let error = Config::from_lookup(lookup_with_upstream(HashMap::from([
            ("TARGET_HOSTNAME", "."),
            ("RELAY_IPV4", "203.0.113.10"),
        ])))
        .expect_err("target hostname should be the DNS root");

        assert_eq!(error, ConfigError::Invalid { key: "TARGET_HOSTNAME", value: ".".to_string() });
    }

    #[test]
    fn invalid_bind_addr_should_be_rejected() {
        let error = Config::from_lookup(lookup_with_upstream(HashMap::from([
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
        let error = Config::from_lookup(lookup_with_upstream(HashMap::from([
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
    fn required_intra_upstream_should_be_loaded() {
        let config = Config::from_lookup(lookup_with_upstream(HashMap::from([
            ("TARGET_HOSTNAME", "example.com"),
            ("RELAY_IPV4", "203.0.113.10"),
            ("INTRA_UPSTREAM_DOH_URL", "https://dns.example.net/custom-query"),
        ])))
        .expect("configuration should be valid");

        assert_eq!(config.intra_upstream_doh_url.as_str(), "https://dns.example.net/custom-query");
    }

    #[test]
    fn non_https_intra_upstream_should_be_rejected() {
        let error = Config::from_lookup(lookup_with_upstream(HashMap::from([
            ("TARGET_HOSTNAME", "example.com"),
            ("RELAY_IPV4", "203.0.113.10"),
            ("INTRA_UPSTREAM_DOH_URL", "http://dns.example.net/dns-query"),
        ])))
        .expect_err("upstream URL should require HTTPS");

        assert_eq!(
            error,
            ConfigError::Invalid {
                key: "INTRA_UPSTREAM_DOH_URL",
                value: "http://dns.example.net/dns-query".to_string(),
            }
        );
    }

    #[test]
    fn intra_upstream_should_be_required() {
        let error = Config::from_lookup(lookup(HashMap::from([
            ("TARGET_HOSTNAME", "example.com"),
            ("RELAY_IPV4", "203.0.113.10"),
        ])))
        .expect_err("upstream URL should be absent");

        assert_eq!(error, ConfigError::Missing { key: "INTRA_UPSTREAM_DOH_URL" });
    }
}
