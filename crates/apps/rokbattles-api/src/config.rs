//! API configuration loaded from environment variables.

use std::env;

/// Runtime settings for the API process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub bind_addr: String,
    pub mongo_uri: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub discord_redirect_uri: String,
    pub dns_check_secret: String,
    pub sentry_dsn: Option<String>,
}

/// Errors for missing or invalid config.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing required env var: {key}")]
    Missing { key: &'static str },
}

impl Config {
    /// Load config from environment variables (`.env` is loaded in `main`).
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| env::var(key).ok())
    }

    fn from_lookup<F>(lookup: F) -> Result<Self, ConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let bind_addr = lookup("BIND_ADDR").unwrap_or_else(|| "0.0.0.0:8001".to_string());
        let mongo_uri = required(&lookup, "MONGODB_URI")?;
        let discord_client_id = required(&lookup, "DISCORD_CLIENT_ID")?;
        let discord_client_secret = required(&lookup, "DISCORD_CLIENT_SECRET")?;
        let discord_redirect_uri = required(&lookup, "DISCORD_REDIRECT_URI")?;
        let dns_check_secret = required_non_empty(&lookup, "DNS_CHECK_SECRET")?;
        let sentry_dsn = lookup("SENTRY_DSN").filter(|value| !value.is_empty());

        Ok(Self {
            bind_addr,
            mongo_uri,
            discord_client_id,
            discord_client_secret,
            discord_redirect_uri,
            dns_check_secret,
            sentry_dsn,
        })
    }
}

fn required<F>(lookup: &F, key: &'static str) -> Result<String, ConfigError>
where
    F: Fn(&str) -> Option<String>,
{
    lookup(key).ok_or(ConfigError::Missing { key })
}

fn required_non_empty<F>(lookup: &F, key: &'static str) -> Result<String, ConfigError>
where
    F: Fn(&str) -> Option<String>,
{
    required(lookup, key).and_then(|value| {
        (!value.trim().is_empty()).then_some(value).ok_or(ConfigError::Missing { key })
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
    fn uses_defaults_for_optional_values() {
        let cfg = Config::from_lookup(lookup(HashMap::from([
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
            ("DISCORD_CLIENT_ID", "discord-client-id"),
            ("DISCORD_CLIENT_SECRET", "discord-client-secret"),
            ("DISCORD_REDIRECT_URI", "https://example.com/proxy/v1/auth/discord/callback"),
            ("DNS_CHECK_SECRET", "dns-check-secret"),
        ])))
        .expect("config");

        assert_eq!(cfg.bind_addr, "0.0.0.0:8001");
        assert_eq!(cfg.mongo_uri, "mongodb://localhost:27017/rokbattles");
        assert_eq!(cfg.discord_client_id, "discord-client-id");
        assert_eq!(cfg.discord_client_secret, "discord-client-secret");
        assert_eq!(cfg.discord_redirect_uri, "https://example.com/proxy/v1/auth/discord/callback");
        assert_eq!(cfg.dns_check_secret, "dns-check-secret");
        assert_eq!(cfg.sentry_dsn, None);
    }

    #[test]
    fn loads_optional_sentry_dsn() {
        let cfg = Config::from_lookup(lookup(HashMap::from([
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
            ("DISCORD_CLIENT_ID", "discord-client-id"),
            ("DISCORD_CLIENT_SECRET", "discord-client-secret"),
            ("DISCORD_REDIRECT_URI", "https://example.com/proxy/v1/auth/discord/callback"),
            ("DNS_CHECK_SECRET", "dns-check-secret"),
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
    fn requires_discord_client_id() {
        let err = Config::from_lookup(lookup(HashMap::from([
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
            ("DISCORD_CLIENT_SECRET", "discord-client-secret"),
            ("DISCORD_REDIRECT_URI", "https://example.com/proxy/v1/auth/discord/callback"),
        ])))
        .expect_err("missing discord client id");
        assert_eq!(err, ConfigError::Missing { key: "DISCORD_CLIENT_ID" });
    }

    #[test]
    fn requires_discord_client_secret() {
        let err = Config::from_lookup(lookup(HashMap::from([
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
            ("DISCORD_CLIENT_ID", "discord-client-id"),
            ("DISCORD_REDIRECT_URI", "https://example.com/proxy/v1/auth/discord/callback"),
        ])))
        .expect_err("missing discord client secret");
        assert_eq!(err, ConfigError::Missing { key: "DISCORD_CLIENT_SECRET" });
    }

    #[test]
    fn requires_discord_redirect_uri() {
        let err = Config::from_lookup(lookup(HashMap::from([
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
            ("DISCORD_CLIENT_ID", "discord-client-id"),
            ("DISCORD_CLIENT_SECRET", "discord-client-secret"),
        ])))
        .expect_err("missing discord redirect uri");
        assert_eq!(err, ConfigError::Missing { key: "DISCORD_REDIRECT_URI" });
    }

    #[test]
    fn requires_dns_check_secret() {
        let err = Config::from_lookup(lookup(HashMap::from([
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
            ("DISCORD_CLIENT_ID", "discord-client-id"),
            ("DISCORD_CLIENT_SECRET", "discord-client-secret"),
            ("DISCORD_REDIRECT_URI", "https://example.com/proxy/v1/auth/discord/callback"),
        ])))
        .expect_err("missing DNS check secret");
        assert_eq!(err, ConfigError::Missing { key: "DNS_CHECK_SECRET" });
    }
}
