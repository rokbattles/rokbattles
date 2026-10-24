use mongodb::bson::{Bson, Document, doc};
use rustc_hash::FxHashMap;

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
}

#[derive(Debug, Clone)]
pub(crate) struct PairingOpponentsRequest {
    pub range: GovernorDateRange,
    pub exclude_activities: Vec<PairingsActivity>,
    pub exclude_battles: Vec<PairingsBattleType>,
    pub primary_commander_id: i64,
    pub secondary_commander_id: i64,
    pub loadout_key: Option<String>,
}

pub(crate) fn parse_pairings_request(
    params: &FxHashMap<String, String>,
) -> Result<PairingsRequest, ApiError> {
    let range = parse_default_governor_date_range(params)?;
    let exclude_activities =
        parse_exclude_activities(params.get("excludeActivities").map(String::as_str))?;
    let exclude_battles = parse_exclude_battles(params.get("excludeBattles").map(String::as_str))?;
    Ok(PairingsRequest { range, exclude_activities, exclude_battles })
}

pub(crate) fn parse_pairing_loadouts_request(
    params: &FxHashMap<String, String>,
) -> Result<PairingLoadoutsRequest, ApiError> {
    let range = parse_default_governor_date_range(params)?;
    let exclude_activities =
        parse_exclude_activities(params.get("excludeActivities").map(String::as_str))?;
    let exclude_battles = parse_exclude_battles(params.get("excludeBattles").map(String::as_str))?;
    let primary_commander_id = parse_positive_required_i64(params, "primary", "Invalid pairing")?;
    let secondary_commander_id =
        parse_non_negative_required_i64(params, "secondary", "Invalid pairing")?;

    Ok(PairingLoadoutsRequest {
        range,
        exclude_activities,
        exclude_battles,
        primary_commander_id,
        secondary_commander_id,
    })
}

pub(crate) fn parse_pairing_opponents_request(
    params: &FxHashMap<String, String>,
) -> Result<PairingOpponentsRequest, ApiError> {
    let range = parse_default_governor_date_range(params)?;
    let exclude_activities =
        parse_exclude_activities(params.get("excludeActivities").map(String::as_str))?;
    let exclude_battles = parse_exclude_battles(params.get("excludeBattles").map(String::as_str))?;
    let primary_commander_id = parse_positive_required_i64(params, "primary", "Invalid pairing")?;
    let secondary_commander_id =
        parse_non_negative_required_i64(params, "secondary", "Invalid pairing")?;
    let loadout_key = params
        .get("loadoutKey")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    Ok(PairingOpponentsRequest {
        range,
        exclude_activities,
        exclude_battles,
        primary_commander_id,
        secondary_commander_id,
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
    exclude_battles.iter().copied().map(battle_type_condition).collect()
}

fn battle_type_condition(battle_type: PairingsBattleType) -> Document {
    match battle_type {
        PairingsBattleType::Garrison => sender_garrison_condition(),
        PairingsBattleType::Rally => doc! {
            "$and": [
                sender_not_garrison_condition(),
                sender_rally_condition(),
            ]
        },
        PairingsBattleType::Swarming => doc! {
            "$and": [
                sender_not_garrison_condition(),
                sender_not_rally_condition(),
                opponent_rally_or_garrison_condition(),
            ]
        },
        PairingsBattleType::OpenField => doc! {
            "$and": [
                sender_not_garrison_condition(),
                sender_not_rally_condition(),
                { "$nor": [opponent_rally_or_garrison_condition()] },
            ]
        },
    }
}

fn sender_garrison_condition() -> Document {
    doc! { "$or": sender_garrison_field_conditions() }
}

fn sender_not_garrison_condition() -> Document {
    doc! { "$nor": sender_garrison_field_conditions() }
}

fn sender_garrison_field_conditions() -> Vec<Document> {
    vec![
        doc! { "sender.alliance_building_id": { "$exists": true, "$ne": Bson::Null } },
        doc! { "sender.structure_id": { "$exists": true, "$ne": Bson::Null } },
    ]
}

fn sender_rally_condition() -> Document {
    doc! { "sender.rally": true }
}

fn sender_not_rally_condition() -> Document {
    doc! { "sender.rally": { "$ne": true } }
}

fn opponent_rally_or_garrison_condition() -> Document {
    doc! {
        "opponents": {
            "$elemMatch": {
                "$or": [
                    { "rally": true },
                    { "alliance_building_id": { "$exists": true, "$ne": Bson::Null } },
                    { "structure_id": { "$exists": true, "$ne": Bson::Null } },
                ]
            }
        }
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

fn parse_positive_required_i64(
    params: &FxHashMap<String, String>,
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
    params: &FxHashMap<String, String>,
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
    fn parse_pairing_loadouts_request_accepts_pairing() {
        let request = parse_pairing_loadouts_request(&FxHashMap::from_iter([
            ("primary".to_string(), "123".to_string()),
            ("secondary".to_string(), "0".to_string()),
        ]))
        .expect("request");
        assert_eq!((request.primary_commander_id, request.secondary_commander_id), (123, 0));
    }

    #[test]
    fn parse_pairing_opponents_request_defaults_to_all_loadouts() {
        let request = parse_pairing_opponents_request(&FxHashMap::from_iter([
            ("primary".to_string(), "123".to_string()),
            ("secondary".to_string(), "456".to_string()),
        ]))
        .expect("request");
        assert_eq!(request.loadout_key, None);
    }

    #[test]
    fn parse_pairing_opponents_request_accepts_loadout_key() {
        let request = parse_pairing_opponents_request(&FxHashMap::from_iter([
            ("primary".to_string(), "123".to_string()),
            ("secondary".to_string(), "456".to_string()),
            ("loadoutKey".to_string(), " eq:1:100_2:25|arm:1000_2|ins:101|fm:1 ".to_string()),
        ]))
        .expect("request");
        assert_eq!(request.loadout_key.as_deref(), Some("eq:1:100_2:25|arm:1000_2|ins:101|fm:1"));
    }

    #[test]
    fn parse_pairing_opponents_request_treats_blank_loadout_key_as_all_loadouts() {
        let request = parse_pairing_opponents_request(&FxHashMap::from_iter([
            ("primary".to_string(), "123".to_string()),
            ("secondary".to_string(), "456".to_string()),
            ("loadoutKey".to_string(), "  ".to_string()),
        ]))
        .expect("request");
        assert_eq!(request.loadout_key, None);
    }

    #[test]
    fn parse_pairings_request_resolves_date_range() {
        let request = parse_pairings_request(&FxHashMap::from_iter([
            ("start".to_string(), "2025-02-03".to_string()),
            ("end".to_string(), "2025-02-04".to_string()),
        ]))
        .expect("request");
        assert_eq!(request.range.start, "2025-02-03");
        assert_eq!(request.range.end, "2025-02-04");
    }

    #[test]
    fn parse_pairings_request_defaults_to_include_all_battles() {
        let request = parse_pairings_request(&FxHashMap::default()).expect("request");

        assert_eq!((request.exclude_activities, request.exclude_battles), (vec![], vec![]));
    }

    #[test]
    fn parse_pairings_request_parses_excluded_activities() {
        let request = parse_pairings_request(&FxHashMap::from_iter([(
            "excludeActivities".to_string(),
            "ark,kvk,ark".to_string(),
        )]))
        .expect("request");
        assert_eq!(request.exclude_activities, vec![PairingsActivity::Ark, PairingsActivity::Kvk]);
    }

    #[test]
    fn parse_pairings_request_parses_excluded_battles() {
        let request = parse_pairings_request(&FxHashMap::from_iter([(
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
        let request = parse_pairings_request(&FxHashMap::from_iter([(
            "excludeActivities".to_string(),
            "unknown".to_string(),
        )]));
        request.expect_err("input should be rejected");
    }

    #[test]
    fn parse_pairings_request_rejects_invalid_excluded_battle() {
        let request = parse_pairings_request(&FxHashMap::from_iter([(
            "excludeBattles".to_string(),
            "duel".to_string(),
        )]));
        request.expect_err("input should be rejected");
    }

    #[test]
    fn garrison_battle_condition_matches_sender_structure_fields() {
        assert_eq!(
            battle_type_condition(PairingsBattleType::Garrison),
            sender_garrison_condition()
        );
    }

    #[test]
    fn rally_battle_condition_excludes_sender_garrisons() {
        assert_eq!(
            battle_type_condition(PairingsBattleType::Rally),
            doc! {
                "$and": [
                    sender_not_garrison_condition(),
                    sender_rally_condition(),
                ]
            }
        );
    }

    #[test]
    fn sender_rally_conditions_use_boolean_schema() {
        assert_eq!(
            (sender_rally_condition(), sender_not_rally_condition()),
            (doc! { "sender.rally": true }, doc! { "sender.rally": { "$ne": true } },)
        );
    }

    #[test]
    fn swarming_battle_condition_requires_special_opponent_only() {
        assert_eq!(
            battle_type_condition(PairingsBattleType::Swarming),
            doc! {
                "$and": [
                    sender_not_garrison_condition(),
                    sender_not_rally_condition(),
                    opponent_rally_or_garrison_condition(),
                ]
            }
        );
    }

    #[test]
    fn open_field_battle_condition_excludes_all_special_marches() {
        assert_eq!(
            battle_type_condition(PairingsBattleType::OpenField),
            doc! {
                "$and": [
                    sender_not_garrison_condition(),
                    sender_not_rally_condition(),
                    { "$nor": [opponent_rally_or_garrison_condition()] },
                ]
            }
        );
    }
}
