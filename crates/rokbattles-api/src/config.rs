//! API configuration loaded from environment variables.

use std::env;

/// Runtime settings for the API process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub bind_addr: String,
    pub mongo_uri: String,
    pub cron_secret: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub discord_redirect_uri: String,
    pub log_filter: String,
}

/// Errors for missing required config.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing required env var: MONGODB_URI")]
    MissingMongoUri,
    #[error("missing required env var: CRON_SECRET")]
    MissingCronSecret,
    #[error("missing required env var: DISCORD_CLIENT_ID")]
    MissingDiscordClientId,
    #[error("missing required env var: DISCORD_CLIENT_SECRET")]
    MissingDiscordClientSecret,
    #[error("missing required env var: DISCORD_REDIRECT_URI")]
    MissingDiscordRedirectUri,
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
        let mongo_uri = lookup("MONGODB_URI").ok_or(ConfigError::MissingMongoUri)?;
        let cron_secret = lookup("CRON_SECRET").ok_or(ConfigError::MissingCronSecret)?;
        let discord_client_id =
            lookup("DISCORD_CLIENT_ID").ok_or(ConfigError::MissingDiscordClientId)?;
        let discord_client_secret =
            lookup("DISCORD_CLIENT_SECRET").ok_or(ConfigError::MissingDiscordClientSecret)?;
        let discord_redirect_uri =
            lookup("DISCORD_REDIRECT_URI").ok_or(ConfigError::MissingDiscordRedirectUri)?;
        let log_filter =
            lookup("RUST_LOG").unwrap_or_else(|| "rokbattles_api=info,axum=info".to_string());

        Ok(Self {
            bind_addr,
            mongo_uri,
            cron_secret,
            discord_client_id,
            discord_client_secret,
            discord_redirect_uri,
            log_filter,
        })
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
    fn uses_defaults_for_optional_values() {
        let cfg = Config::from_lookup(lookup(HashMap::from([
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
            ("CRON_SECRET", "test-secret"),
            ("DISCORD_CLIENT_ID", "discord-client-id"),
            ("DISCORD_CLIENT_SECRET", "discord-client-secret"),
            ("DISCORD_REDIRECT_URI", "https://example.com/proxy/v1/auth/discord/callback"),
        ])))
        .expect("config");

        assert_eq!(cfg.bind_addr, "0.0.0.0:8001");
        assert_eq!(cfg.log_filter, "rokbattles_api=info,axum=info");
        assert_eq!(cfg.mongo_uri, "mongodb://localhost:27017/rokbattles");
        assert_eq!(cfg.cron_secret, "test-secret");
        assert_eq!(cfg.discord_client_id, "discord-client-id");
        assert_eq!(cfg.discord_client_secret, "discord-client-secret");
        assert_eq!(cfg.discord_redirect_uri, "https://example.com/proxy/v1/auth/discord/callback");
    }

    #[test]
    fn requires_mongo_uri() {
        let err = Config::from_lookup(lookup(HashMap::new())).expect_err("missing uri");
        assert_eq!(err, ConfigError::MissingMongoUri);
    }

    #[test]
    fn requires_cron_secret() {
        let err = Config::from_lookup(lookup(HashMap::from([(
            "MONGODB_URI",
            "mongodb://localhost:27017/rokbattles",
        )])))
        .expect_err("missing cron secret");
        assert_eq!(err, ConfigError::MissingCronSecret);
    }

    #[test]
    fn requires_discord_client_id() {
        let err = Config::from_lookup(lookup(HashMap::from([
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
            ("CRON_SECRET", "test-secret"),
            ("DISCORD_CLIENT_SECRET", "discord-client-secret"),
            ("DISCORD_REDIRECT_URI", "https://example.com/proxy/v1/auth/discord/callback"),
        ])))
        .expect_err("missing discord client id");
        assert_eq!(err, ConfigError::MissingDiscordClientId);
    }

    #[test]
    fn requires_discord_client_secret() {
        let err = Config::from_lookup(lookup(HashMap::from([
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
            ("CRON_SECRET", "test-secret"),
            ("DISCORD_CLIENT_ID", "discord-client-id"),
            ("DISCORD_REDIRECT_URI", "https://example.com/proxy/v1/auth/discord/callback"),
        ])))
        .expect_err("missing discord client secret");
        assert_eq!(err, ConfigError::MissingDiscordClientSecret);
    }

    #[test]
    fn requires_discord_redirect_uri() {
        let err = Config::from_lookup(lookup(HashMap::from([
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
            ("CRON_SECRET", "test-secret"),
            ("DISCORD_CLIENT_ID", "discord-client-id"),
            ("DISCORD_CLIENT_SECRET", "discord-client-secret"),
        ])))
        .expect_err("missing discord redirect uri");
        assert_eq!(err, ConfigError::MissingDiscordRedirectUri);
    }
}
