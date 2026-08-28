//! Environment-driven gateway configuration for the DNS-over-HTTPS resolver.

use std::{collections::HashSet, env, net::Ipv4Addr};

use crate::resolver::is_public_unicast;

/// Runtime configuration loaded from environment variables.
#[derive(Debug, PartialEq, Eq)]
pub struct Config {
    /// Ordered public IPv4 addresses returned for the game hostname.
    pub gateway: Vec<Ipv4Addr>,
    /// Authenticated API endpoint that records successful DNS canaries.
    pub dns_check_callback_url: String,
    /// Shared secret used only for resolver-to-API canary callbacks.
    pub dns_check_secret: String,
}

/// Errors returned when gateway configuration is missing or invalid.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    /// The required gateway list was not set.
    #[error("missing required env var: GATEWAY")]
    Missing,
    /// Another required value was not set.
    #[error("missing required env var: {key}")]
    MissingValue { key: &'static str },
    /// A required value was present but empty.
    #[error("env var must not be empty: {key}")]
    Empty { key: &'static str },
    /// The list or one of its comma-separated fields was empty.
    #[error("GATEWAY must contain only non-empty comma-separated IPv4 addresses")]
    EmptyAddress,
    /// One field was not an IPv4 address.
    #[error("invalid IPv4 address in GATEWAY: {value}")]
    InvalidAddress { value: String },
    /// Client-facing gateway answers must be publicly routable unicast.
    #[error("gateway IPv4 address is not public unicast: {address}")]
    NonPublic { address: Ipv4Addr },
    /// Repeating an address does not add a gateway node.
    #[error("duplicate gateway IPv4 address: {address}")]
    Duplicate { address: Ipv4Addr },
    /// The finite environment value could not be reserved safely.
    #[error("GATEWAY is too large for available memory")]
    Allocation,
}

impl Config {
    /// Load the gateway fleet from `GATEWAY`.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] when `GATEWAY` is absent, empty, contains a
    /// duplicate, cannot be reserved, or contains an invalid or
    /// non-public-unicast IPv4 address.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| env::var(key).ok())
    }

    fn from_lookup<F>(lookup: F) -> Result<Self, ConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let value = lookup("GATEWAY").ok_or(ConfigError::Missing)?;
        let dns_check_callback_url = required(&lookup, "DNS_CHECK_CALLBACK_URL")?;
        let dns_check_secret = required(&lookup, "DNS_CHECK_SECRET")?;
        Ok(Self { gateway: parse_gateway_ipv4s(&value)?, dns_check_callback_url, dns_check_secret })
    }
}

fn required<F>(lookup: &F, key: &'static str) -> Result<String, ConfigError>
where
    F: Fn(&str) -> Option<String>,
{
    let value = lookup(key).ok_or(ConfigError::MissingValue { key })?;
    if value.trim().is_empty() {
        return Err(ConfigError::Empty { key });
    }
    Ok(value)
}

fn parse_gateway_ipv4s(value: &str) -> Result<Vec<Ipv4Addr>, ConfigError> {
    let field_count = value.bytes().filter(|byte| *byte == b',').count().saturating_add(1);
    let mut addresses = Vec::new();
    let mut seen = HashSet::new();
    addresses.try_reserve(field_count).map_err(|_| ConfigError::Allocation)?;
    seen.try_reserve(field_count).map_err(|_| ConfigError::Allocation)?;
    for field in value.split(',').map(str::trim) {
        if field.is_empty() {
            return Err(ConfigError::EmptyAddress);
        }
        let address =
            field.parse().map_err(|_| ConfigError::InvalidAddress { value: field.to_string() })?;
        if !is_public_unicast(address) {
            return Err(ConfigError::NonPublic { address });
        }
        if !seen.insert(address) {
            return Err(ConfigError::Duplicate { address });
        }
        addresses.push(address);
    }
    if addresses.is_empty() {
        return Err(ConfigError::EmptyAddress);
    }
    Ok(addresses)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup(gateway: Option<&str>) -> impl Fn(&str) -> Option<String> {
        let gateway = gateway.map(str::to_string);
        move |key| match key {
            "GATEWAY" => gateway.clone(),
            "DNS_CHECK_CALLBACK_URL" => {
                Some("https://rokbattles.com/proxy/v1/dns-check/mark".into())
            }
            "DNS_CHECK_SECRET" => Some("test-secret".into()),
            _ => None,
        }
    }

    #[test]
    fn gateway_should_be_required() {
        let error = Config::from_lookup(lookup(None)).expect_err("gateway should be absent");

        assert_eq!(error, ConfigError::Missing);
    }

    #[test]
    fn gateway_should_load_trimmed_addresses_in_order() {
        let config = Config::from_lookup(lookup(Some("93.184.216.34, 1.1.1.1")))
            .expect("configuration should be valid");

        assert_eq!(config.gateway, [Ipv4Addr::new(93, 184, 216, 34), Ipv4Addr::new(1, 1, 1, 1)]);
        assert_eq!(config.dns_check_callback_url, "https://rokbattles.com/proxy/v1/dns-check/mark");
        assert_eq!(config.dns_check_secret, "test-secret");
    }

    #[test]
    fn gateway_should_not_have_an_application_node_count_limit() {
        let value = (11..=32)
            .map(|first_octet| format!("{first_octet}.0.0.1"))
            .collect::<Vec<_>>()
            .join(",");
        let config = Config::from_lookup(move |key| match key {
            "GATEWAY" => Some(value.clone()),
            "DNS_CHECK_CALLBACK_URL" => Some("https://example.com/mark".into()),
            "DNS_CHECK_SECRET" => Some("test-secret".into()),
            _ => None,
        })
        .expect("configuration should accept more than the former eight-node limit");

        assert_eq!(config.gateway.len(), 22);
    }

    #[test]
    fn duplicate_gateway_address_should_be_rejected() {
        let address = Ipv4Addr::new(93, 184, 216, 34);
        let error = Config::from_lookup(lookup(Some("93.184.216.34,93.184.216.34")))
            .expect_err("duplicate gateway should be rejected");

        assert_eq!(error, ConfigError::Duplicate { address });
    }

    #[test]
    fn empty_gateway_address_should_be_rejected() {
        let error = Config::from_lookup(lookup(Some("93.184.216.34,,1.1.1.1")))
            .expect_err("empty gateway address should be rejected");

        assert_eq!(error, ConfigError::EmptyAddress);
    }

    #[test]
    fn non_ipv4_gateway_address_should_be_rejected() {
        let value = "2001:db8::1";
        let error = Config::from_lookup(lookup(Some(value)))
            .expect_err("gateway should contain only IPv4 addresses");

        assert_eq!(error, ConfigError::InvalidAddress { value: value.to_string() });
    }

    #[test]
    fn non_public_gateway_address_should_be_rejected() {
        let address = Ipv4Addr::LOCALHOST;
        let error = Config::from_lookup(lookup(Some("127.0.0.1")))
            .expect_err("gateway should be public unicast");

        assert_eq!(error, ConfigError::NonPublic { address });
    }

    #[test]
    fn dns_check_callback_and_secret_should_be_required() {
        let missing_callback = Config::from_lookup(|key| match key {
            "GATEWAY" => Some("93.184.216.34".into()),
            _ => None,
        })
        .expect_err("callback should be required");
        assert_eq!(missing_callback, ConfigError::MissingValue { key: "DNS_CHECK_CALLBACK_URL" });

        let missing_secret = Config::from_lookup(|key| match key {
            "GATEWAY" => Some("93.184.216.34".into()),
            "DNS_CHECK_CALLBACK_URL" => Some("https://example.com/mark".into()),
            _ => None,
        })
        .expect_err("secret should be required");
        assert_eq!(missing_secret, ConfigError::MissingValue { key: "DNS_CHECK_SECRET" });
    }
}
