use std::collections::HashMap;

use crate::error::ApiError;
use crate::routes::reports::common::query::parse_optional_i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReportsFilterType {
    Home,
    Ark,
    Kvk,
    Strife,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReportsFilterSide {
    None,
    Sender,
    Opponent,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReportsGarrisonBuildingType {
    Flag,
    Fortress,
    Other,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ReportsRequest {
    pub before_cursor: Option<i64>,
    pub after_cursor: Option<i64>,
    pub filter_type: Option<ReportsFilterType>,
    pub player_id: Option<i64>,
    pub sender_primary_commander_id: Option<i64>,
    pub sender_secondary_commander_id: Option<i64>,
    pub opponent_primary_commander_id: Option<i64>,
    pub opponent_secondary_commander_id: Option<i64>,
    pub rally_side: ReportsFilterSide,
    pub garrison_side: ReportsFilterSide,
    pub garrison_building_type: Option<ReportsGarrisonBuildingType>,
}

impl ReportsRequest {
    pub fn sort_direction(&self) -> i32 {
        if self.before_cursor.is_some() { 1 } else { -1 }
    }
}

pub(crate) fn parse_reports_request(
    params: &HashMap<String, String>,
) -> Result<ReportsRequest, ApiError> {
    let before_cursor = parse_optional_i64(
        params.get("before").map(String::as_str),
        "Invalid before cursor",
    )?;
    let after_cursor = if before_cursor.is_some() {
        None
    } else {
        parse_optional_i64(
            params.get("after").map(String::as_str),
            "Invalid after cursor",
        )?
    };

    let filter_type = parse_filter_type(params.get("type").map(String::as_str))?;
    let player_id =
        parse_optional_i64(params.get("pid").map(String::as_str), "Invalid governor id")?;
    let sender_primary_commander_id = parse_optional_i64(
        params.get("spc").map(String::as_str),
        "Invalid sender primary commander id",
    )?;
    let sender_secondary_commander_id = parse_optional_i64(
        params.get("ssc").map(String::as_str),
        "Invalid sender secondary commander id",
    )?;
    let opponent_primary_commander_id = parse_optional_i64(
        params.get("opc").map(String::as_str),
        "Invalid opponent primary commander id",
    )?;
    let opponent_secondary_commander_id = parse_optional_i64(
        params.get("osc").map(String::as_str),
        "Invalid opponent secondary commander id",
    )?;

    let rally_side = parse_filter_side(params.get("rs").map(String::as_str), "Invalid rally side")?
        .unwrap_or(ReportsFilterSide::None);
    let garrison_side = parse_filter_side(
        params.get("gs").map(String::as_str),
        "Invalid garrison side",
    )?
    .unwrap_or(ReportsFilterSide::None);

    if sides_overlap(rally_side, garrison_side) {
        return Err(ApiError::bad_request(
            "Rally and garrison cannot overlap on the same side",
        ));
    }

    let garrison_building_type = parse_garrison_building_type(
        params.get("gb").map(String::as_str),
        "Invalid garrison building",
    )?;

    Ok(ReportsRequest {
        before_cursor,
        after_cursor,
        filter_type,
        player_id,
        sender_primary_commander_id,
        sender_secondary_commander_id,
        opponent_primary_commander_id,
        opponent_secondary_commander_id,
        rally_side,
        garrison_side,
        garrison_building_type,
    })
}

fn parse_filter_type(value: Option<&str>) -> Result<Option<ReportsFilterType>, ApiError> {
    let Some(value) = value else {
        return Ok(None);
    };

    let parsed = match value {
        "home" => ReportsFilterType::Home,
        "ark" => ReportsFilterType::Ark,
        "kvk" => ReportsFilterType::Kvk,
        "strife" => ReportsFilterType::Strife,
        _ => return Err(ApiError::bad_request("Invalid type")),
    };

    Ok(Some(parsed))
}

fn parse_filter_side(
    value: Option<&str>,
    error_message: &'static str,
) -> Result<Option<ReportsFilterSide>, ApiError> {
    let Some(value) = value else {
        return Ok(None);
    };

    let parsed = match value {
        "none" => ReportsFilterSide::None,
        "sender" => ReportsFilterSide::Sender,
        "opponent" => ReportsFilterSide::Opponent,
        "both" => ReportsFilterSide::Both,
        _ => return Err(ApiError::bad_request(error_message)),
    };

    Ok(Some(parsed))
}

fn parse_garrison_building_type(
    value: Option<&str>,
    error_message: &'static str,
) -> Result<Option<ReportsGarrisonBuildingType>, ApiError> {
    let Some(value) = value else {
        return Ok(None);
    };

    let parsed = match value {
        "flag" => ReportsGarrisonBuildingType::Flag,
        "fortress" => ReportsGarrisonBuildingType::Fortress,
        "other" => ReportsGarrisonBuildingType::Other,
        _ => return Err(ApiError::bad_request(error_message)),
    };

    Ok(Some(parsed))
}

fn sides_overlap(a: ReportsFilterSide, b: ReportsFilterSide) -> bool {
    (selection_has_side(a, ReportsFilterSide::Sender)
        && selection_has_side(b, ReportsFilterSide::Sender))
        || (selection_has_side(a, ReportsFilterSide::Opponent)
            && selection_has_side(b, ReportsFilterSide::Opponent))
}

fn selection_has_side(selection: ReportsFilterSide, side: ReportsFilterSide) -> bool {
    matches!(
        (selection, side),
        (ReportsFilterSide::Both, ReportsFilterSide::Sender)
            | (ReportsFilterSide::Both, ReportsFilterSide::Opponent)
            | (ReportsFilterSide::Sender, ReportsFilterSide::Sender)
            | (ReportsFilterSide::Opponent, ReportsFilterSide::Opponent)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_filter_side() {
        let parsed = parse_reports_request(&HashMap::from([
            ("rs".to_string(), "sender".to_string()),
            ("gs".to_string(), "none".to_string()),
        ]))
        .expect("parsed request");
        assert_eq!(parsed.rally_side, ReportsFilterSide::Sender);
    }

    #[test]
    fn rejects_non_numeric_cursor() {
        let result =
            parse_reports_request(&HashMap::from([("after".to_string(), "abc".to_string())]));
        assert!(result.is_err());
    }

    #[test]
    fn rejects_overlapping_sides() {
        let result = parse_reports_request(&HashMap::from([
            ("rs".to_string(), "both".to_string()),
            ("gs".to_string(), "sender".to_string()),
        ]));
        assert!(result.is_err());
    }
}
