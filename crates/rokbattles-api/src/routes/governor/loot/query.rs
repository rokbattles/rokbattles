use std::collections::HashMap;

use crate::{
    error::ApiError,
    routes::governor::{common::parse_default_governor_date_range, date_range::GovernorDateRange},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BarbarianLootNpc {
    Barbarians,
    Marauders,
}

#[derive(Debug, Clone)]
pub(crate) struct BarbarianLootRequest {
    pub range: GovernorDateRange,
    pub npc: BarbarianLootNpc,
    pub levels: Vec<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FortLootNpc {
    BarbarianForts,
    MarauderEncampments,
}

#[derive(Debug, Clone)]
pub(crate) struct FortLootRequest {
    pub range: GovernorDateRange,
    pub npc: FortLootNpc,
    pub level: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BaulurLootNpc {
    IronhandBaulur,
    MiserKhaolak,
}

#[derive(Debug, Clone)]
pub(crate) struct BaulurLootRequest {
    pub range: GovernorDateRange,
    pub npc: BaulurLootNpc,
}

pub(crate) fn parse_barbarian_loot_request(
    params: &HashMap<String, String>,
) -> Result<BarbarianLootRequest, ApiError> {
    let range = parse_default_governor_date_range(params)?;
    let npc = match params.get("type").map(|value| value.trim()).filter(|value| !value.is_empty()) {
        Some("barbarians") | None => BarbarianLootNpc::Barbarians,
        Some("marauders") => BarbarianLootNpc::Marauders,
        Some(_) => return Err(ApiError::bad_request("Invalid type")),
    };
    let levels = parse_levels(params, "level")?;
    validate_barbarian_levels(npc, &levels)?;

    Ok(BarbarianLootRequest { range, npc, levels })
}

pub(crate) fn parse_fort_loot_request(
    params: &HashMap<String, String>,
) -> Result<FortLootRequest, ApiError> {
    let range = parse_default_governor_date_range(params)?;
    let npc = match params.get("type").map(|value| value.trim()).filter(|value| !value.is_empty()) {
        Some("barbarian-forts") | None => FortLootNpc::BarbarianForts,
        Some("marauder-encampments") => FortLootNpc::MarauderEncampments,
        Some(_) => return Err(ApiError::bad_request("Invalid type")),
    };
    let levels = parse_levels(params, "level")?;
    if levels.len() > 1 {
        return Err(ApiError::bad_request("Only one level can be selected"));
    }
    let level = levels.first().copied();
    validate_fort_level(npc, level)?;

    Ok(FortLootRequest { range, npc, level })
}

pub(crate) fn parse_baulur_loot_request(
    params: &HashMap<String, String>,
) -> Result<BaulurLootRequest, ApiError> {
    let range = parse_default_governor_date_range(params)?;
    let npc = match params.get("type").map(|value| value.trim()).filter(|value| !value.is_empty()) {
        Some("ironhand-baulur") | None => BaulurLootNpc::IronhandBaulur,
        Some("miser-khaolak") => BaulurLootNpc::MiserKhaolak,
        Some(_) => return Err(ApiError::bad_request("Invalid type")),
    };

    Ok(BaulurLootRequest { range, npc })
}

fn parse_levels(params: &HashMap<String, String>, key: &str) -> Result<Vec<i32>, ApiError> {
    let Some(raw) = params.get(key).map(|value| value.trim()).filter(|value| !value.is_empty())
    else {
        return Ok(Vec::new());
    };

    let mut levels = Vec::new();
    for value in raw.split(',').map(str::trim).filter(|value| !value.is_empty()) {
        let level =
            value.parse::<i32>().map_err(|_| ApiError::bad_request(format!("Invalid {key}")))?;
        if !levels.contains(&level) {
            levels.push(level);
        }
    }
    levels.sort_unstable();

    Ok(levels)
}

fn validate_barbarian_levels(npc: BarbarianLootNpc, levels: &[i32]) -> Result<(), ApiError> {
    for level in levels {
        let valid = match npc {
            BarbarianLootNpc::Barbarians => (1..=55).contains(level),
            BarbarianLootNpc::Marauders => matches!(level, 1 | 41),
        };
        if !valid {
            return Err(ApiError::bad_request("Invalid level"));
        }
    }

    Ok(())
}

fn validate_fort_level(npc: FortLootNpc, level: Option<i32>) -> Result<(), ApiError> {
    let Some(level) = level else {
        return Ok(());
    };
    let valid = match npc {
        FortLootNpc::BarbarianForts => (1..=15).contains(&level),
        FortLootNpc::MarauderEncampments => matches!(level, 1 | 11),
    };
    if valid { Ok(()) } else { Err(ApiError::bad_request("Invalid level")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_barbarian_loot_request_defaults_to_barbarians_all_levels() {
        let request = parse_barbarian_loot_request(&date_params()).expect("request");

        assert_eq!(request.npc, BarbarianLootNpc::Barbarians);
        assert!(request.levels.is_empty());
        assert_eq!(request.range.start, "2025-02-03");
        assert_eq!(request.range.end, "2025-02-04");
    }

    #[test]
    fn parse_barbarian_loot_request_allows_multiple_distinct_levels() {
        let mut params = date_params();
        params.insert("level".to_string(), "41,1,41".to_string());

        let request = parse_barbarian_loot_request(&params).expect("request");

        assert_eq!(request.levels, vec![1, 41]);
    }

    #[test]
    fn parse_barbarian_loot_request_rejects_invalid_marauder_level() {
        let mut params = date_params();
        params.insert("type".to_string(), "marauders".to_string());
        params.insert("level".to_string(), "2".to_string());

        assert!(parse_barbarian_loot_request(&params).is_err());
    }

    #[test]
    fn parse_fort_loot_request_rejects_multiple_levels() {
        let mut params = date_params();
        params.insert("level".to_string(), "1,2".to_string());

        assert!(parse_fort_loot_request(&params).is_err());
    }

    #[test]
    fn parse_baulur_loot_request_defaults_to_ironhand_baulur() {
        let request = parse_baulur_loot_request(&date_params()).expect("request");

        assert_eq!(request.npc, BaulurLootNpc::IronhandBaulur);
    }

    fn date_params() -> HashMap<String, String> {
        HashMap::from([
            ("start".to_string(), "2025-02-03".to_string()),
            ("end".to_string(), "2025-02-04".to_string()),
        ])
    }
}
