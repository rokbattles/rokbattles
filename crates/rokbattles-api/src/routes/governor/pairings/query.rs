use std::collections::HashMap;

use crate::error::ApiError;
use crate::routes::governor::common::parse_positive_governor_id_str;
use crate::routes::governor::date_range::{GovernorDateRange, parse_governor_date_range};

const DEFAULT_MAX_RANGE_DAYS: i64 = 366;

#[derive(Debug, Clone)]
pub(crate) struct PairingsRequest {
    pub range: GovernorDateRange,
}

#[derive(Debug, Clone)]
pub(crate) struct PairingLoadoutsRequest {
    pub range: GovernorDateRange,
    pub primary_commander_id: i64,
    pub secondary_commander_id: i64,
    pub granularity: LoadoutGranularity,
}

#[derive(Debug, Clone)]
pub(crate) struct PairingOpponentsRequest {
    pub range: GovernorDateRange,
    pub primary_commander_id: i64,
    pub secondary_commander_id: i64,
    pub granularity: OpponentGranularity,
    pub loadout_key: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoadoutGranularity {
    Simplified,
    Exact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpponentGranularity {
    Overall,
    Simplified,
    Exact,
}

pub(crate) fn parse_governor_id(raw_governor_id: &str) -> Result<i64, ApiError> {
    parse_positive_governor_id_str(raw_governor_id)
        .ok_or_else(|| ApiError::bad_request("Invalid governorId"))
}

pub(crate) fn parse_pairings_request(
    params: &HashMap<String, String>,
) -> Result<PairingsRequest, ApiError> {
    let range = parse_governor_date_range(params, DEFAULT_MAX_RANGE_DAYS)?;
    Ok(PairingsRequest { range })
}

pub(crate) fn parse_pairing_loadouts_request(
    params: &HashMap<String, String>,
) -> Result<PairingLoadoutsRequest, ApiError> {
    let range = parse_governor_date_range(params, DEFAULT_MAX_RANGE_DAYS)?;
    let primary_commander_id = parse_positive_required_i64(params, "primary", "Invalid pairing")?;
    let secondary_commander_id =
        parse_non_negative_required_i64(params, "secondary", "Invalid pairing")?;
    let granularity = parse_loadout_granularity(params.get("granularity").map(String::as_str))?;

    Ok(PairingLoadoutsRequest {
        range,
        primary_commander_id,
        secondary_commander_id,
        granularity,
    })
}

pub(crate) fn parse_pairing_opponents_request(
    params: &HashMap<String, String>,
) -> Result<PairingOpponentsRequest, ApiError> {
    let range = parse_governor_date_range(params, DEFAULT_MAX_RANGE_DAYS)?;
    let primary_commander_id = parse_positive_required_i64(params, "primary", "Invalid pairing")?;
    let secondary_commander_id =
        parse_non_negative_required_i64(params, "secondary", "Invalid pairing")?;
    let granularity = parse_opponent_granularity(params.get("granularity").map(String::as_str))?;
    let loadout_key = params
        .get("loadoutKey")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if granularity != OpponentGranularity::Overall && loadout_key.is_none() {
        return Err(ApiError::bad_request("Missing loadoutKey"));
    }

    Ok(PairingOpponentsRequest {
        range,
        primary_commander_id,
        secondary_commander_id,
        granularity,
        loadout_key,
    })
}

fn parse_loadout_granularity(raw: Option<&str>) -> Result<LoadoutGranularity, ApiError> {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(LoadoutGranularity::Exact),
        Some("simplified") => Ok(LoadoutGranularity::Simplified),
        Some("exact") => Ok(LoadoutGranularity::Exact),
        Some("normalized") => Err(ApiError::bad_request("Invalid granularity")),
        Some(_) => Err(ApiError::bad_request("Invalid granularity")),
    }
}

fn parse_opponent_granularity(raw: Option<&str>) -> Result<OpponentGranularity, ApiError> {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(OpponentGranularity::Overall),
        Some("overall") => Ok(OpponentGranularity::Overall),
        Some("simplified") => Ok(OpponentGranularity::Simplified),
        Some("exact") => Ok(OpponentGranularity::Exact),
        Some("normalized") => Err(ApiError::bad_request("Invalid granularity")),
        Some(_) => Err(ApiError::bad_request("Invalid granularity")),
    }
}

fn parse_positive_required_i64(
    params: &HashMap<String, String>,
    key: &str,
    error: &str,
) -> Result<i64, ApiError> {
    params
        .get(key)
        .map(String::as_str)
        .map(str::trim)
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| ApiError::bad_request(error))
}

fn parse_non_negative_required_i64(
    params: &HashMap<String, String>,
    key: &str,
    error: &str,
) -> Result<i64, ApiError> {
    params
        .get(key)
        .map(String::as_str)
        .map(str::trim)
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value >= 0)
        .ok_or_else(|| ApiError::bad_request(error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_governor_id_accepts_positive_integer() {
        assert_eq!(parse_governor_id("123").expect("governor id"), 123);
        assert_eq!(parse_governor_id(" 42 ").expect("governor id"), 42);
    }

    #[test]
    fn parse_governor_id_rejects_invalid_values() {
        assert!(parse_governor_id("").is_err());
        assert!(parse_governor_id("0").is_err());
        assert!(parse_governor_id("-1").is_err());
        assert!(parse_governor_id("abc").is_err());
    }

    #[test]
    fn parse_pairing_loadouts_request_rejects_normalized_granularity() {
        let request = parse_pairing_loadouts_request(&HashMap::from([
            ("primary".to_string(), "123".to_string()),
            ("secondary".to_string(), "456".to_string()),
            ("granularity".to_string(), "normalized".to_string()),
        ]));
        assert!(request.is_err());
    }

    #[test]
    fn parse_pairing_opponents_request_requires_loadout_key_for_non_overall() {
        let request = parse_pairing_opponents_request(&HashMap::from([
            ("primary".to_string(), "123".to_string()),
            ("secondary".to_string(), "456".to_string()),
            ("granularity".to_string(), "simplified".to_string()),
        ]));
        assert!(request.is_err());
    }

    #[test]
    fn parse_pairings_request_resolves_date_range() {
        let request = parse_pairings_request(&HashMap::from([
            ("start".to_string(), "2025-02-03".to_string()),
            ("end".to_string(), "2025-02-04".to_string()),
        ]))
        .expect("request");
        assert_eq!(request.range.start, "2025-02-03");
        assert_eq!(request.range.end, "2025-02-04");
    }
}
