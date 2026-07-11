use std::collections::HashMap;

use mongodb::bson::{Bson, Document, doc};

use crate::{
    error::ApiError,
    routes::governor::{common::parse_default_governor_date_range, date_range::GovernorDateRange},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PairingsReportType {
    Home,
    Ark,
    Kvk,
    Strife,
}

#[derive(Debug, Clone)]
pub(crate) struct PairingsRequest {
    pub range: GovernorDateRange,
    pub exclude_types: Vec<PairingsReportType>,
}

#[derive(Debug, Clone)]
pub(crate) struct PairingLoadoutsRequest {
    pub range: GovernorDateRange,
    pub exclude_types: Vec<PairingsReportType>,
    pub primary_commander_id: i64,
    pub secondary_commander_id: i64,
    pub granularity: LoadoutGranularity,
}

#[derive(Debug, Clone)]
pub(crate) struct PairingOpponentsRequest {
    pub range: GovernorDateRange,
    pub exclude_types: Vec<PairingsReportType>,
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

pub(crate) fn parse_pairings_request(
    params: &HashMap<String, String>,
) -> Result<PairingsRequest, ApiError> {
    let range = parse_default_governor_date_range(params)?;
    let exclude_types = parse_exclude_types(params.get("excludeTypes").map(String::as_str))?;
    Ok(PairingsRequest { range, exclude_types })
}

pub(crate) fn parse_pairing_loadouts_request(
    params: &HashMap<String, String>,
) -> Result<PairingLoadoutsRequest, ApiError> {
    let range = parse_default_governor_date_range(params)?;
    let exclude_types = parse_exclude_types(params.get("excludeTypes").map(String::as_str))?;
    let primary_commander_id = parse_positive_required_i64(params, "primary", "Invalid pairing")?;
    let secondary_commander_id =
        parse_non_negative_required_i64(params, "secondary", "Invalid pairing")?;
    let granularity = parse_loadout_granularity(params.get("granularity").map(String::as_str))?;

    Ok(PairingLoadoutsRequest {
        range,
        exclude_types,
        primary_commander_id,
        secondary_commander_id,
        granularity,
    })
}

pub(crate) fn parse_pairing_opponents_request(
    params: &HashMap<String, String>,
) -> Result<PairingOpponentsRequest, ApiError> {
    let range = parse_default_governor_date_range(params)?;
    let exclude_types = parse_exclude_types(params.get("excludeTypes").map(String::as_str))?;
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
        exclude_types,
        primary_commander_id,
        secondary_commander_id,
        granularity,
        loadout_key,
    })
}

pub(crate) fn build_excluded_report_type_conditions(
    exclude_types: &[PairingsReportType],
) -> Vec<Document> {
    exclude_types
        .iter()
        .copied()
        .map(|filter_type| match filter_type {
            PairingsReportType::Kvk => doc! { "metadata.kvk": true },
            PairingsReportType::Ark => doc! { "metadata.mail_role": "dungeon" },
            PairingsReportType::Home => doc! {
                "$and": [
                    { "metadata.kvk": { "$ne": true } },
                    { "metadata.mail_role": { "$ne": "dungeon" } },
                    {
                        "$or": [
                            { "sender.supreme_strife.battle_id": { "$in": [Bson::Null, Bson::String(String::new())] } },
                            { "sender.supreme_strife.team_id": { "$in": [Bson::Null, Bson::Int32(0), Bson::Int64(0)] } },
                        ]
                    }
                ]
            },
            PairingsReportType::Strife => doc! {
                "$and": [
                    { "sender.supreme_strife.battle_id": { "$exists": true, "$nin": [Bson::Null, Bson::String(String::new())] } },
                    { "sender.supreme_strife.team_id": { "$exists": true, "$nin": [Bson::Null, Bson::Int32(0), Bson::Int64(0)] } },
                ]
            },
        })
        .collect()
}

fn parse_exclude_types(raw: Option<&str>) -> Result<Vec<PairingsReportType>, ApiError> {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(Vec::new());
    };

    let mut exclude_types = Vec::new();

    for value in raw.split(',').map(str::trim).filter(|value| !value.is_empty()) {
        let parsed = match value {
            "home" => PairingsReportType::Home,
            "ark" => PairingsReportType::Ark,
            "kvk" => PairingsReportType::Kvk,
            "strife" => PairingsReportType::Strife,
            _ => return Err(ApiError::bad_request("Invalid excludeTypes")),
        };

        if !exclude_types.contains(&parsed) {
            exclude_types.push(parsed);
        }
    }

    Ok(exclude_types)
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

    #[test]
    fn parse_pairings_request_parses_exclude_types() {
        let request = parse_pairings_request(&HashMap::from([(
            "excludeTypes".to_string(),
            "ark,kvk,ark".to_string(),
        )]))
        .expect("request");
        assert_eq!(request.exclude_types, vec![PairingsReportType::Ark, PairingsReportType::Kvk]);
    }

    #[test]
    fn parse_pairings_request_rejects_invalid_exclude_type() {
        let request = parse_pairings_request(&HashMap::from([(
            "excludeTypes".to_string(),
            "unknown".to_string(),
        )]));
        assert!(request.is_err());
    }
}
