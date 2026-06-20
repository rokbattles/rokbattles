//! Environment-driven processor configuration.

use std::{collections::BTreeSet, env, time::Duration};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub mongo_uri: String,
    pub sentry_dsn: Option<String>,
    pub batch_size: i64,
    pub idle_sleep: Duration,
    pub api_filter: ApiFilter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiFilter {
    pub enabled: bool,
    pub allowed_api_ids: BTreeSet<u32>,
}

impl ApiFilter {
    pub fn accepts(&self, api_id: u32) -> bool {
        !self.enabled || self.allowed_api_ids.contains(&api_id)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing required env var: {key}")]
    Missing { key: &'static str },
    #[error("invalid value for {key}: {value}")]
    Invalid { key: &'static str, value: String },
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| env::var(key).ok())
    }

    fn from_lookup<F>(lookup: F) -> Result<Self, ConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        Ok(Self {
            mongo_uri: required(&lookup, "MONGODB_URI")?,
            sentry_dsn: lookup("SENTRY_DSN").filter(|value| !value.is_empty()),
            batch_size: parse_i64(
                "TCP_PROCESSOR_BATCH_SIZE",
                lookup("TCP_PROCESSOR_BATCH_SIZE"),
                25,
            )?,
            idle_sleep: parse_duration_secs(
                "TCP_PROCESSOR_IDLE_SLEEP_SECS",
                lookup("TCP_PROCESSOR_IDLE_SLEEP_SECS"),
                15,
            )?,
            api_filter: ApiFilter {
                enabled: parse_bool(
                    "TCP_PROCESSOR_API_FILTER_ENABLED",
                    lookup("TCP_PROCESSOR_API_FILTER_ENABLED"),
                    false,
                )?,
                allowed_api_ids: parse_api_ids(lookup("TCP_PROCESSOR_ALLOWED_API_IDS"))?,
            },
        })
    }
}

fn required<F>(lookup: &F, key: &'static str) -> Result<String, ConfigError>
where
    F: Fn(&str) -> Option<String>,
{
    lookup(key).ok_or(ConfigError::Missing { key })
}

fn parse_bool(
    key: &'static str,
    value: Option<String>,
    default: bool,
) -> Result<bool, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(ConfigError::Invalid { key, value }),
    }
}

fn parse_i64(key: &'static str, value: Option<String>, default: i64) -> Result<i64, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    let parsed = value.parse::<i64>().map_err(|_error| ConfigError::Invalid { key, value })?;
    if parsed <= 0 {
        return Err(ConfigError::Invalid { key, value: parsed.to_string() });
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
    let parsed = value.parse::<u64>().map_err(|_error| ConfigError::Invalid { key, value })?;
    if parsed == 0 {
        return Err(ConfigError::Invalid { key, value: parsed.to_string() });
    }
    Ok(Duration::from_secs(parsed))
}

fn parse_api_ids(value: Option<String>) -> Result<BTreeSet<u32>, ConfigError> {
    let Some(value) = value else {
        return Ok(BTreeSet::new());
    };

    let mut ids = BTreeSet::new();
    for item in value.split(',').map(str::trim).filter(|item| !item.is_empty()) {
        let id = item.parse::<u32>().map_err(|_error| ConfigError::Invalid {
            key: "TCP_PROCESSOR_ALLOWED_API_IDS",
            value: item.to_string(),
        })?;
        ids.insert(id);
    }
    Ok(ids)
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
        let cfg = Config::from_lookup(lookup(HashMap::from([(
            "MONGODB_URI",
            "mongodb://localhost:27017/rokbattles",
        )])))
        .expect("config");

        assert_eq!(
            cfg,
            Config {
                mongo_uri: "mongodb://localhost:27017/rokbattles".to_string(),
                sentry_dsn: None,
                batch_size: 25,
                idle_sleep: Duration::from_secs(15),
                api_filter: ApiFilter { enabled: false, allowed_api_ids: BTreeSet::new() },
            }
        );
    }

    #[test]
    fn loads_optional_sentry_dsn() {
        let cfg = Config::from_lookup(lookup(HashMap::from([
            ("MONGODB_URI", "mongodb://localhost:27017/rokbattles"),
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
    fn api_filter_accepts_all_when_disabled() {
        let filter = ApiFilter { enabled: false, allowed_api_ids: BTreeSet::new() };

        assert!(filter.accepts(8562));
    }

    #[test]
    fn api_filter_checks_allow_list_when_enabled() {
        let filter = ApiFilter { enabled: true, allowed_api_ids: BTreeSet::from([8562, 14]) };

        assert!(filter.accepts(14));
        assert!(!filter.accepts(9999));
    }

    #[test]
    fn parse_api_ids_accepts_comma_list() {
        let ids = parse_api_ids(Some("14, 8562".to_string())).expect("ids should parse");

        assert_eq!(ids, BTreeSet::from([14, 8562]));
    }
}
