//! Read startup settings without resolving hosts or changing process state.
//!
//! `BIND_ADDR` defaults to `0.0.0.0:3101`; `UPSTREAM_ADDR` and `RELAY_TOKEN` are
//! required. Parsing checks the bind socket, a nonempty upstream host and nonzero
//! port, and a token containing non-whitespace characters. DNS resolution and
//! upstream address restrictions belong to [`crate::service::resolve`].
//!
//! [`Config`]'s debug output deliberately omits the bearer token.

use std::{env, net::SocketAddrV4};

const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3101";

/// Startup settings for NAT rules and authenticated mail uploads.
///
/// The constructors validate settings. Public fields permit direct construction,
/// so consumers must not assume an arbitrary `Config` passed those checks.
#[derive(Clone, PartialEq, Eq)]
pub struct Config {
    /// Local destination IPv4 address and TCP port selected for forwarding.
    /// An unspecified address selects all local IPv4 destinations on that port.
    pub bind_addr: SocketAddrV4,
    /// Upstream host and nonzero TCP port, resolved once at startup.
    pub upstream_addr: String,
    /// Ingress bearer token, preserved verbatim and omitted from `Debug` output.
    pub relay_token: String,
}

impl Config {
    /// Read settings from the process environment.
    ///
    /// Non-Unicode values are treated as absent, as are missing variables.
    /// See [`Self::from_lookup`] for defaults and validation.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] when a required value is absent or invalid.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| env::var(key).ok())
    }

    /// Parse settings supplied by `lookup`, without accessing the environment.
    ///
    /// `BIND_ADDR` must be an IPv4 socket with a nonzero port. Upstream validation
    /// only separates the final port from a nonempty host; DNS and address-family
    /// validation are deferred to [`crate::service::resolve`]. Tokens are checked
    /// for blankness but are not trimmed before use.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] for missing required settings, an invalid bind
    /// socket or upstream port, or a blank token.
    ///
    /// # Examples
    ///
    /// ```
    /// use rokbattles_nat_gateway::config::Config;
    ///
    /// let config = Config::from_lookup(|key| match key {
    ///     "UPSTREAM_ADDR" => Some("game.example:3101".into()),
    ///     "RELAY_TOKEN" => Some("example-token".into()),
    ///     _ => None,
    /// })?;
    /// assert!(config.bind_addr.ip().is_unspecified());
    /// assert_eq!(config.bind_addr.port(), 3101);
    /// # Ok::<(), rokbattles_nat_gateway::config::ConfigError>(())
    /// ```
    pub fn from_lookup<F>(lookup: F) -> Result<Self, ConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let bind_addr = lookup("BIND_ADDR").unwrap_or_else(|| DEFAULT_BIND_ADDR.to_string());
        let bind_addr = parse_address("BIND_ADDR", bind_addr)?;
        let upstream_addr = required(&lookup, "UPSTREAM_ADDR")?;
        let relay_token = required(&lookup, "RELAY_TOKEN")?;

        if !valid_host_and_port(&upstream_addr) {
            return Err(ConfigError::InvalidUpstream { value: upstream_addr });
        }
        if relay_token.trim().is_empty() {
            return Err(ConfigError::Empty { key: "RELAY_TOKEN" });
        }

        Ok(Self { bind_addr, upstream_addr, relay_token })
    }
}

/// A setting that cannot be used to construct startup configuration.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    /// A required environment variable was absent.
    #[error("missing required environment variable {key}")]
    Missing {
        /// Name of the absent setting.
        key: &'static str,
    },
    /// A required environment variable was empty.
    #[error("environment variable {key} must not be empty")]
    Empty {
        /// Name of the blank setting.
        key: &'static str,
    },
    /// An endpoint was not an explicit IPv4 socket address.
    #[error("environment variable {key} must be an IPv4 address and nonzero port: {value}")]
    InvalidAddress {
        /// Name of the setting containing the bind socket.
        key: &'static str,
        /// Rejected address text.
        value: String,
    },
    /// The upstream did not contain a host and nonzero port.
    #[error("UPSTREAM_ADDR must contain a host and nonzero port: {value}")]
    InvalidUpstream {
        /// Rejected upstream text.
        value: String,
    },
}

fn required<F>(lookup: &F, key: &'static str) -> Result<String, ConfigError>
where
    F: Fn(&str) -> Option<String>,
{
    lookup(key).ok_or(ConfigError::Missing { key })
}

fn parse_address(key: &'static str, value: String) -> Result<SocketAddrV4, ConfigError> {
    let address = value
        .parse::<SocketAddrV4>()
        .map_err(|_error| ConfigError::InvalidAddress { key, value: value.clone() })?;
    if address.port() == 0 {
        return Err(ConfigError::InvalidAddress { key, value });
    }
    Ok(address)
}

// Leave host syntax and DNS errors to the resolver; only the port is parsed here.
fn valid_host_and_port(value: &str) -> bool {
    let Some((host, port)) = value.rsplit_once(':') else {
        return false;
    };
    !host.trim().is_empty() && port.parse::<u16>().is_ok_and(|port| port != 0)
}

impl std::fmt::Debug for Config {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Config")
            .field("bind_addr", &self.bind_addr)
            .field("upstream_addr", &self.upstream_addr)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::*;

    fn lookup(entries: &[(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let values = entries.iter().copied().collect::<FxHashMap<_, _>>();
        move |key| values.get(key).map(|value| (*value).to_string())
    }

    #[test]
    fn loads_explicit_ipv4_endpoints_and_token() {
        let config = Config::from_lookup(lookup(&[
            ("BIND_ADDR", "192.0.2.10:3101"),
            ("UPSTREAM_ADDR", "198.51.100.20:3101"),
            ("RELAY_TOKEN", "secret"),
        ]))
        .expect("configuration should be valid");

        assert_eq!(config.bind_addr, "192.0.2.10:3101".parse().expect("test address"));
        assert_eq!(config.upstream_addr, "198.51.100.20:3101");
        assert_eq!(config.relay_token, "secret");
    }

    #[test]
    fn requires_every_value() {
        let error = Config::from_lookup(lookup(&[])).expect_err("upstream address is required");

        assert_eq!(error, ConfigError::Missing { key: "UPSTREAM_ADDR" });
    }

    #[test]
    fn rejects_ipv6_and_zero_bind_ports() {
        for (bind, expected) in [
            (
                "[2001:db8::1]:3101",
                ConfigError::InvalidAddress {
                    key: "BIND_ADDR",
                    value: "[2001:db8::1]:3101".to_string(),
                },
            ),
            (
                "192.0.2.10:0",
                ConfigError::InvalidAddress { key: "BIND_ADDR", value: "192.0.2.10:0".to_string() },
            ),
        ] {
            let error = Config::from_lookup(lookup(&[
                ("BIND_ADDR", bind),
                ("UPSTREAM_ADDR", "upstream.example:3101"),
                ("RELAY_TOKEN", "secret"),
            ]))
            .expect_err("configuration should be rejected");
            assert_eq!(error, expected);
        }
    }

    #[test]
    fn defaults_to_all_local_ipv4_addresses() {
        let config = Config::from_lookup(lookup(&[
            ("UPSTREAM_ADDR", "rocgate.lilithgame.com:3101"),
            ("RELAY_TOKEN", "secret"),
        ]))
        .expect("default bind address should be valid");

        assert_eq!(config.bind_addr, "0.0.0.0:3101".parse().expect("test address"));
    }

    #[test]
    fn rejects_an_empty_token() {
        let error = Config::from_lookup(lookup(&[
            ("BIND_ADDR", "192.0.2.10:3101"),
            ("UPSTREAM_ADDR", "198.51.100.20:3101"),
            ("RELAY_TOKEN", "  "),
        ]))
        .expect_err("token should be nonempty");

        assert_eq!(error, ConfigError::Empty { key: "RELAY_TOKEN" });
    }

    #[test]
    fn malformed_upstream_ports_and_empty_hosts_are_rejected() {
        for upstream in [
            "",
            "game.example",
            ":3101",
            " :3101",
            "game.example:0",
            "game.example:65536",
            "game.example:-1",
            "game.example:http",
        ] {
            let error = Config::from_lookup(lookup(&[
                ("UPSTREAM_ADDR", upstream),
                ("RELAY_TOKEN", "secret"),
            ]))
            .expect_err("invalid upstream");
            assert_eq!(error, ConfigError::InvalidUpstream { value: upstream.into() });
        }
    }

    #[test]
    fn token_is_preserved_verbatim_and_omitted_from_debug_output() {
        let token = "  private-token  ";
        let config = Config::from_lookup(lookup(&[
            ("UPSTREAM_ADDR", "game.example:3101"),
            ("RELAY_TOKEN", token),
        ]))
        .expect("valid settings");
        assert_eq!(config.relay_token, token);
        let debug = format!("{config:?}");
        assert!(!debug.contains("private-token"));
        assert!(!debug.contains("relay_token"));
    }
}
