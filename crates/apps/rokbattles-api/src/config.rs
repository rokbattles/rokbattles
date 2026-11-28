//! API configuration loaded from environment variables.

use std::env;

use crate::rate_limit::RateLimitMode;

/// Runtime settings for the API process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub bind_addr: String,
    pub rate_limit_mode: RateLimitMode,
    pub mongo_uri: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub discord_redirect_uri: String,
    pub pk_token: Option<String>,
    pub kdmt_token: Option<String>,
    pub turnstile_secret_key: Option<String>,
    pub turnstile_hostnames: Vec<String>,
}

/// Errors for missing or invalid config.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing required env var: {key}")]
    Missing { key: &'static str },
    #[error("invalid env var {key}: expected {expected}")]
    Invalid { key: &'static str, expected: &'static str },
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
        let rate_limit_mode = required(&lookup, "RATE_LIMIT_MODE")?.parse().map_err(|_error| {
            ConfigError::Invalid { key: "RATE_LIMIT_MODE", expected: "local or cloudflare" }
        })?;
        let pk_token = lookup("PK_TOKEN").filter(|value| !value.trim().is_empty());
        let kdmt_token = lookup("KDMT_TOKEN").filter(|value| !value.trim().is_empty());
        let turnstile_secret_key =
            lookup("TURNSTILE_SECRET_KEY").filter(|value| !value.trim().is_empty());
        let turnstile_hostnames = lookup("TURNSTILE_HOSTNAMES")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect();

        Ok(Self {
            bind_addr,
            rate_limit_mode,
            mongo_uri,
            discord_client_id,
            discord_client_secret,
            discord_redirect_uri,
            pk_token,
            kdmt_token,
            turnstile_secret_key,
            turnstile_hostnames,
        })
    }
}

fn required<F>(lookup: &F, key: &'static str) -> Result<String, ConfigError>
where
    F: Fn(&str) -> Option<String>,
{
    lookup(key).ok_or(ConfigError::Missing { key })
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::*;

    fn lookup(vars: FxHashMap<&'static str, &'static str>) -> impl Fn(&str) -> Option<String> {
        move |key| vars.get(key).map(|value| (*value).to_string())
    }

    #[test]
    fn uses_defaults_for_optional_values() {
        let cfg = Config::from_lookup(lookup(FxHashMap::from_iter([
            ("RATE_LIMIT_MODE", "local"),
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
            ("DISCORD_CLIENT_ID", "discord-client-id"),
            ("DISCORD_CLIENT_SECRET", "discord-client-secret"),
            ("DISCORD_REDIRECT_URI", "https://example.com/proxy/v1/auth/discord/callback"),
        ])))
        .expect("config");

        assert_eq!(cfg.bind_addr, "0.0.0.0:8001");
        assert_eq!(cfg.mongo_uri, "mongodb://localhost:27017/rokbattles");
        assert_eq!(cfg.discord_client_id, "discord-client-id");
        assert_eq!(cfg.discord_client_secret, "discord-client-secret");
        assert_eq!(cfg.discord_redirect_uri, "https://example.com/proxy/v1/auth/discord/callback");
        assert_eq!(cfg.pk_token, None);
        assert_eq!(cfg.kdmt_token, None);
        assert_eq!(cfg.turnstile_secret_key, None);
        assert!(cfg.turnstile_hostnames.is_empty());
    }

    #[test]
    fn loads_pk_token_and_disables_access_for_blank_values() {
        for (token, expected) in
            [("partner-secret", Some("partner-secret")), ("", None), ("  ", None)]
        {
            let cfg = Config::from_lookup(lookup(FxHashMap::from_iter([
                ("RATE_LIMIT_MODE", "local"),
                ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
                ("DISCORD_CLIENT_ID", "client"),
                ("DISCORD_CLIENT_SECRET", "secret"),
                ("DISCORD_REDIRECT_URI", "https://example.com/callback"),
                ("PK_TOKEN", token),
            ])))
            .expect("config");

            assert_eq!(cfg.pk_token.as_deref(), expected);
        }
    }

    #[test]
    fn loads_kdmt_independently_and_disables_blank_values() {
        for (token, expected) in [("kdmt-secret", Some("kdmt-secret")), ("", None), ("  ", None)] {
            let cfg = Config::from_lookup(lookup(FxHashMap::from_iter([
                ("RATE_LIMIT_MODE", "local"),
                ("MONGODB_URI", "mongodb://localhost/rokbattles"),
                ("DISCORD_CLIENT_ID", "client"),
                ("DISCORD_CLIENT_SECRET", "secret"),
                ("DISCORD_REDIRECT_URI", "https://example.com/callback"),
                ("PK_TOKEN", "pk-secret"),
                ("KDMT_TOKEN", token),
            ])))
            .expect("config");
            assert_eq!(cfg.kdmt_token.as_deref(), expected);
            assert_eq!(cfg.pk_token.as_deref(), Some("pk-secret"));
        }
    }

    fn with_mode(mode: Option<&str>) -> Result<Config, ConfigError> {
        Config::from_lookup(|key| match key {
            "MONGODB_URI" => Some("mongodb://localhost/rokbattles".to_string()),
            "DISCORD_CLIENT_ID" => Some("client".to_string()),
            "DISCORD_CLIENT_SECRET" => Some("secret".to_string()),
            "DISCORD_REDIRECT_URI" => Some("https://example.com/callback".to_string()),
            "RATE_LIMIT_MODE" => mode.map(str::to_string),
            _ => None,
        })
    }

    #[test]
    fn loads_local_and_cloudflare_rate_limit_modes() {
        for (value, expected) in
            [("local", RateLimitMode::Local), ("cloudflare", RateLimitMode::Cloudflare)]
        {
            assert_eq!(with_mode(Some(value)).expect("config").rate_limit_mode, expected);
        }
    }

    #[test]
    fn missing_rate_limit_mode_cannot_silently_disable_protection() {
        let error = with_mode(None).expect_err("explicit mode required");
        assert_eq!(error, ConfigError::Missing { key: "RATE_LIMIT_MODE" });
    }

    #[test]
    fn rejects_blank_or_unknown_rate_limit_modes() {
        for value in ["", "Cloudflare", "cloudflrae", "disabled", " local "] {
            let error = with_mode(Some(value)).expect_err("invalid mode");
            assert_eq!(
                error,
                ConfigError::Invalid { key: "RATE_LIMIT_MODE", expected: "local or cloudflare" }
            );
        }
    }

    #[test]
    fn requires_mongo_uri() {
        let err = Config::from_lookup(lookup(FxHashMap::default())).expect_err("missing uri");
        assert_eq!(err, ConfigError::Missing { key: "MONGODB_URI" });
    }

    #[test]
    fn requires_discord_client_id() {
        let err = Config::from_lookup(lookup(FxHashMap::from_iter([
            ("RATE_LIMIT_MODE", "local"),
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
            ("DISCORD_CLIENT_SECRET", "discord-client-secret"),
            ("DISCORD_REDIRECT_URI", "https://example.com/proxy/v1/auth/discord/callback"),
        ])))
        .expect_err("missing discord client id");
        assert_eq!(err, ConfigError::Missing { key: "DISCORD_CLIENT_ID" });
    }

    #[test]
    fn requires_discord_client_secret() {
        let err = Config::from_lookup(lookup(FxHashMap::from_iter([
            ("RATE_LIMIT_MODE", "local"),
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
            ("DISCORD_CLIENT_ID", "discord-client-id"),
            ("DISCORD_REDIRECT_URI", "https://example.com/proxy/v1/auth/discord/callback"),
        ])))
        .expect_err("missing discord client secret");
        assert_eq!(err, ConfigError::Missing { key: "DISCORD_CLIENT_SECRET" });
    }

    #[test]
    fn requires_discord_redirect_uri() {
        let err = Config::from_lookup(lookup(FxHashMap::from_iter([
            ("RATE_LIMIT_MODE", "local"),
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
            ("DISCORD_CLIENT_ID", "discord-client-id"),
            ("DISCORD_CLIENT_SECRET", "discord-client-secret"),
        ])))
        .expect_err("missing discord redirect uri");
        assert_eq!(err, ConfigError::Missing { key: "DISCORD_REDIRECT_URI" });
    }
}
