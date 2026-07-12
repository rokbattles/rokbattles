use std::collections::HashMap;

use mongodb::bson::{Bson, Document, doc};

use crate::{
    error::ApiError,
    routes::governor::{common::parse_default_governor_date_range, date_range::GovernorDateRange},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PairingsActivity {
    Home,
    Ark,
    Kvk,
    Strife,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PairingsBattleType {
    OpenField,
    Swarming,
    Rally,
    Garrison,
}

#[derive(Debug, Clone)]
pub(crate) struct PairingsRequest {
    pub range: GovernorDateRange,
    pub exclude_activities: Vec<PairingsActivity>,
    pub exclude_battles: Vec<PairingsBattleType>,
}

#[derive(Debug, Clone)]
pub(crate) struct PairingLoadoutsRequest {
    pub range: GovernorDateRange,
    pub exclude_activities: Vec<PairingsActivity>,
    pub exclude_battles: Vec<PairingsBattleType>,
    pub primary_commander_id: i64,
    pub secondary_commander_id: i64,
    pub granularity: LoadoutGranularity,
}

#[derive(Debug, Clone)]
pub(crate) struct PairingOpponentsRequest {
    pub range: GovernorDateRange,
    pub exclude_activities: Vec<PairingsActivity>,
    pub exclude_battles: Vec<PairingsBattleType>,
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
    let exclude_activities =
        parse_exclude_activities(params.get("excludeActivities").map(String::as_str))?;
    let exclude_battles = parse_exclude_battles(params.get("excludeBattles").map(String::as_str))?;
    Ok(PairingsRequest { range, exclude_activities, exclude_battles })
}

pub(crate) fn parse_pairing_loadouts_request(
    params: &HashMap<String, String>,
) -> Result<PairingLoadoutsRequest, ApiError> {
    let range = parse_default_governor_date_range(params)?;
    let exclude_activities =
        parse_exclude_activities(params.get("excludeActivities").map(String::as_str))?;
    let exclude_battles = parse_exclude_battles(params.get("excludeBattles").map(String::as_str))?;
    let primary_commander_id = parse_positive_required_i64(params, "primary", "Invalid pairing")?;
    let secondary_commander_id =
        parse_non_negative_required_i64(params, "secondary", "Invalid pairing")?;
    let granularity = parse_loadout_granularity(params.get("granularity").map(String::as_str))?;

    Ok(PairingLoadoutsRequest {
        range,
        exclude_activities,
        exclude_battles,
        primary_commander_id,
        secondary_commander_id,
        granularity,
    })
}

pub(crate) fn parse_pairing_opponents_request(
    params: &HashMap<String, String>,
) -> Result<PairingOpponentsRequest, ApiError> {
    let range = parse_default_governor_date_range(params)?;
    let exclude_activities =
        parse_exclude_activities(params.get("excludeActivities").map(String::as_str))?;
    let exclude_battles = parse_exclude_battles(params.get("excludeBattles").map(String::as_str))?;
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
        exclude_activities,
        exclude_battles,
        primary_commander_id,
        secondary_commander_id,
        granularity,
        loadout_key,
    })
}

pub(crate) fn build_excluded_activity_conditions(
    exclude_activities: &[PairingsActivity],
) -> Vec<Document> {
    exclude_activities
        .iter()
        .copied()
        .map(|filter_type| match filter_type {
            PairingsActivity::Kvk => doc! { "metadata.kvk": true },
            PairingsActivity::Ark => doc! { "metadata.mail_role": "dungeon" },
            PairingsActivity::Home => doc! {
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
            PairingsActivity::Strife => doc! {
                "$and": [
                    { "sender.supreme_strife.battle_id": { "$exists": true, "$nin": [Bson::Null, Bson::String(String::new())] } },
                    { "sender.supreme_strife.team_id": { "$exists": true, "$nin": [Bson::Null, Bson::Int32(0), Bson::Int64(0)] } },
                ]
            },
        })
        .collect()
}

pub(crate) fn build_excluded_battle_type_conditions(
    exclude_battles: &[PairingsBattleType],
) -> Vec<Document> {
    exclude_battles
        .iter()
        .copied()
        .map(|battle_type| doc! { "$expr": battle_type_expression(battle_type) })
        .collect()
}

fn battle_type_expression(battle_type: PairingsBattleType) -> Document {
    let sender_garrison = sender_garrison_expression();
    let sender_rally = sender_rally_expression();
    let opponent_rally_or_garrison = opponent_rally_or_garrison_expression();

    match battle_type {
        PairingsBattleType::Garrison => sender_garrison,
        PairingsBattleType::Rally => doc! {
            "$and": [
                { "$not": [sender_garrison] },
                sender_rally,
            ]
        },
        PairingsBattleType::Swarming => doc! {
            "$and": [
                { "$not": [sender_garrison] },
                { "$not": [sender_rally] },
                opponent_rally_or_garrison,
            ]
        },
        PairingsBattleType::OpenField => doc! {
            "$and": [
                { "$not": [sender_garrison] },
                { "$not": [sender_rally] },
                { "$not": [opponent_rally_or_garrison] },
            ]
        },
    }
}

fn sender_garrison_expression() -> Document {
    doc! {
        "$or": [
            { "$ne": [{ "$ifNull": ["$sender.alliance_building_id", Bson::Null] }, Bson::Null] },
            { "$ne": [{ "$ifNull": ["$sender.structure_id", Bson::Null] }, Bson::Null] },
        ]
    }
}

fn sender_rally_expression() -> Document {
    doc! {
        "$in": ["$sender.rally", [Bson::Boolean(true), Bson::Int32(1), Bson::Int64(1)]]
    }
}

fn opponent_rally_or_garrison_expression() -> Document {
    doc! {
        "$gt": [
            {
                "$size": {
                    "$filter": {
                        "input": { "$ifNull": ["$opponents", []] },
                        "as": "opponent",
                        "cond": {
                            "$or": [
                                { "$in": ["$$opponent.rally", [Bson::Boolean(true), Bson::Int32(1), Bson::Int64(1)]] },
                                { "$ne": [{ "$ifNull": ["$$opponent.alliance_building_id", Bson::Null] }, Bson::Null] },
                                { "$ne": [{ "$ifNull": ["$$opponent.structure_id", Bson::Null] }, Bson::Null] },
                            ]
                        }
                    }
                }
            },
            0,
        ]
    }
}

fn parse_exclude_activities(raw: Option<&str>) -> Result<Vec<PairingsActivity>, ApiError> {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(Vec::new());
    };

    let mut exclude_activities = Vec::new();

    for value in raw.split(',').map(str::trim).filter(|value| !value.is_empty()) {
        let parsed = match value {
            "home" => PairingsActivity::Home,
            "ark" => PairingsActivity::Ark,
            "kvk" => PairingsActivity::Kvk,
            "strife" => PairingsActivity::Strife,
            _ => return Err(ApiError::bad_request("Invalid excludeActivities")),
        };

        if !exclude_activities.contains(&parsed) {
            exclude_activities.push(parsed);
        }
    }

    Ok(exclude_activities)
}

fn parse_exclude_battles(raw: Option<&str>) -> Result<Vec<PairingsBattleType>, ApiError> {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(Vec::new());
    };

    let mut exclude_battles = Vec::new();

    for value in raw.split(',').map(str::trim).filter(|value| !value.is_empty()) {
        let parsed = match value {
            "open-field" => PairingsBattleType::OpenField,
            "swarming" => PairingsBattleType::Swarming,
            "rally" => PairingsBattleType::Rally,
            "garrison" => PairingsBattleType::Garrison,
            _ => return Err(ApiError::bad_request("Invalid excludeBattles")),
        };

        if !exclude_battles.contains(&parsed) {
            exclude_battles.push(parsed);
        }
    }

    Ok(exclude_battles)
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
    fn parse_pairings_request_defaults_to_include_all_battles() {
        let request = parse_pairings_request(&HashMap::new()).expect("request");

        assert_eq!((request.exclude_activities, request.exclude_battles), (vec![], vec![]));
    }

    #[test]
    fn parse_pairings_request_parses_excluded_activities() {
        let request = parse_pairings_request(&HashMap::from([(
            "excludeActivities".to_string(),
            "ark,kvk,ark".to_string(),
        )]))
        .expect("request");
        assert_eq!(request.exclude_activities, vec![PairingsActivity::Ark, PairingsActivity::Kvk]);
    }

    #[test]
    fn parse_pairings_request_parses_excluded_battles() {
        let request = parse_pairings_request(&HashMap::from([(
            "excludeBattles".to_string(),
            "open-field,swarming,rally,garrison,swarming".to_string(),
        )]))
        .expect("request");

        assert_eq!(
            request.exclude_battles,
            vec![
                PairingsBattleType::OpenField,
                PairingsBattleType::Swarming,
                PairingsBattleType::Rally,
                PairingsBattleType::Garrison,
            ]
        );
    }

    #[test]
    fn parse_pairings_request_rejects_invalid_excluded_activity() {
        let request = parse_pairings_request(&HashMap::from([(
            "excludeActivities".to_string(),
            "unknown".to_string(),
        )]));
        assert!(request.is_err());
    }

    #[test]
    fn parse_pairings_request_rejects_invalid_excluded_battle() {
        let request = parse_pairings_request(&HashMap::from([(
            "excludeBattles".to_string(),
            "duel".to_string(),
        )]));
        assert!(request.is_err());
    }

    #[test]
    fn garrison_battle_condition_matches_sender_structure_fields() {
        assert_eq!(
            battle_type_expression(PairingsBattleType::Garrison),
            sender_garrison_expression()
        );
    }

    #[test]
    fn rally_battle_condition_excludes_sender_garrisons() {
        assert_eq!(
            battle_type_expression(PairingsBattleType::Rally),
            doc! {
                "$and": [
                    { "$not": [sender_garrison_expression()] },
                    sender_rally_expression(),
                ]
            }
        );
    }

    #[test]
    fn swarming_battle_condition_requires_special_opponent_only() {
        assert_eq!(
            battle_type_expression(PairingsBattleType::Swarming),
            doc! {
                "$and": [
                    { "$not": [sender_garrison_expression()] },
                    { "$not": [sender_rally_expression()] },
                    opponent_rally_or_garrison_expression(),
                ]
            }
        );
    }

    #[test]
    fn open_field_battle_condition_excludes_all_special_marches() {
        assert_eq!(
            battle_type_expression(PairingsBattleType::OpenField),
            doc! {
                "$and": [
                    { "$not": [sender_garrison_expression()] },
                    { "$not": [sender_rally_expression()] },
                    { "$not": [opponent_rally_or_garrison_expression()] },
                ]
            }
        );
    }
}
