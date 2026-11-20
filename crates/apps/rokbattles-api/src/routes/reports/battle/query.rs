use rustc_hash::FxHashMap;

use crate::{error::ApiError, routes::reports::common::query::parse_optional_i64};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReportsFilterType {
    Home,
    Ark,
    Kvk,
    Strife,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReportsFilterSubtype {
    KvkSeason1,
    KvkSeason2,
    KvkSeason3,
    KvkSeasonOfConquest,
    ArkGoldenBattleground,
    ArkSilverBattleground,
    ArkOsirisLeague,
    ArkPracticeMatch,
    ArkCustomMatch,
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

#[derive(Debug, Clone)]
pub(crate) struct ReportsRequest {
    pub before_cursor: Option<i64>,
    pub after_cursor: Option<i64>,
    pub filter_type: Option<ReportsFilterType>,
    pub filter_subtype: Option<ReportsFilterSubtype>,
    pub player_id: Option<i64>,
    pub receiver_id: Option<i64>,
    pub sender_primary_commander_id: Option<i64>,
    pub sender_secondary_commander_id: Option<i64>,
    pub opponent_primary_commander_id: Option<i64>,
    pub opponent_secondary_commander_id: Option<i64>,
    /// Commander pair that may be on either side of the battle.
    pub commander_primary_id: Option<i64>,
    pub commander_secondary_id: Option<i64>,
    /// Primary commander on either side must be one of these (used for troop type).
    pub commander_primary_ids: Vec<i64>,
    /// Trade % bounds, inclusive.
    pub trade_min: Option<i64>,
    pub trade_max: Option<i64>,
    /// Report time bounds in unix seconds, inclusive.
    pub time_from: Option<i64>,
    pub time_to: Option<i64>,
    pub rally_side: ReportsFilterSide,
    pub garrison_side: ReportsFilterSide,
    pub garrison_building_type: Option<ReportsGarrisonBuildingType>,
    /// Only battles that are neither rallies nor garrisons.
    pub open_field: bool,
}

impl ReportsRequest {
    pub fn sort_direction(&self) -> i32 {
        if self.before_cursor.is_some() { 1 } else { -1 }
    }
}

pub(crate) fn parse_reports_request(
    params: &FxHashMap<String, String>,
) -> Result<ReportsRequest, ApiError> {
    let before_cursor =
        parse_optional_i64(params.get("before").map(String::as_str), "Invalid before cursor")?;
    let after_cursor = if before_cursor.is_some() {
        None
    } else {
        parse_optional_i64(params.get("after").map(String::as_str), "Invalid after cursor")?
    };

    let filter_type = parse_filter_type(params.get("type").map(String::as_str))?;
    let filter_subtype =
        parse_filter_subtype(filter_type, params.get("subtype").map(String::as_str))?;
    let player_id =
        parse_optional_i64(params.get("pid").map(String::as_str), "Invalid governor id")?;
    let receiver_id = parse_optional_i64(
        params.get("receiver").map(String::as_str),
        "Invalid receiver governor id",
    )?;
    if receiver_id.is_some_and(|id| id <= 0) {
        return Err(ApiError::bad_request("Invalid receiver governor id"));
    }
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

    let commander_primary_id =
        parse_optional_i64(params.get("cpc").map(String::as_str), "Invalid commander id")?;
    let commander_secondary_id =
        parse_optional_i64(params.get("csc").map(String::as_str), "Invalid commander id")?;

    let commander_primary_ids = parse_id_list(params.get("cps").map(String::as_str))?;

    let trade_min = parse_trade_bound(params.get("tmin").map(String::as_str))?;
    let trade_max = parse_trade_bound(params.get("tmax").map(String::as_str))?;
    if let (Some(min), Some(max)) = (trade_min, trade_max)
        && min > max
    {
        return Err(ApiError::bad_request("Trade minimum cannot exceed the maximum"));
    }

    let time_from = parse_unix_seconds(params.get("tfrom").map(String::as_str))?;
    let time_to = parse_unix_seconds(params.get("tto").map(String::as_str))?;
    if let (Some(from), Some(to)) = (time_from, time_to)
        && from > to
    {
        return Err(ApiError::bad_request("Start time cannot be after the end time"));
    }

    let rally_side = parse_filter_side(params.get("rs").map(String::as_str), "Invalid rally side")?
        .unwrap_or(ReportsFilterSide::None);
    let garrison_side =
        parse_filter_side(params.get("gs").map(String::as_str), "Invalid garrison side")?
            .unwrap_or(ReportsFilterSide::None);

    if sides_overlap(rally_side, garrison_side) {
        return Err(ApiError::bad_request("Rally and garrison cannot overlap on the same side"));
    }

    let garrison_building_type = parse_garrison_building_type(
        params.get("gb").map(String::as_str),
        "Invalid garrison building",
    )?;

    let open_field = matches!(params.get("of").map(String::as_str), Some("1"));
    if open_field
        && (rally_side != ReportsFilterSide::None
            || garrison_side != ReportsFilterSide::None
            || garrison_building_type.is_some())
    {
        return Err(ApiError::bad_request("Open field cannot be combined with rally or garrison"));
    }

    Ok(ReportsRequest {
        before_cursor,
        after_cursor,
        filter_type,
        filter_subtype,
        player_id,
        receiver_id,
        sender_primary_commander_id,
        sender_secondary_commander_id,
        opponent_primary_commander_id,
        opponent_secondary_commander_id,
        commander_primary_id,
        commander_secondary_id,
        commander_primary_ids,
        trade_min,
        trade_max,
        time_from,
        time_to,
        rally_side,
        garrison_side,
        garrison_building_type,
        open_field,
    })
}

const MAX_COMMANDER_IDS: usize = 500;
const MAX_TRADE_PERCENT: i64 = 1_000_000;

fn parse_id_list(value: Option<&str>) -> Result<Vec<i64>, ApiError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };

    let ids = value
        .split(',')
        .map(|part| match part.trim().parse::<i64>() {
            Ok(id) if id > 0 => Ok(id),
            _ => Err(ApiError::bad_request("Invalid commander id list")),
        })
        .collect::<Result<Vec<_>, _>>()?;

    if ids.len() > MAX_COMMANDER_IDS {
        return Err(ApiError::bad_request("Too many commander ids"));
    }

    Ok(ids)
}

/// Unix seconds up to the year 2100, so the microsecond conversion cannot overflow.
const MAX_UNIX_SECONDS: i64 = 4_102_444_800;

fn parse_unix_seconds(value: Option<&str>) -> Result<Option<i64>, ApiError> {
    let Some(value) = value else {
        return Ok(None);
    };

    match value.parse::<i64>() {
        Ok(seconds) if (0..=MAX_UNIX_SECONDS).contains(&seconds) => Ok(Some(seconds)),
        _ => Err(ApiError::bad_request("Invalid time")),
    }
}

fn parse_trade_bound(value: Option<&str>) -> Result<Option<i64>, ApiError> {
    let Some(value) = value else {
        return Ok(None);
    };

    match value.parse::<i64>() {
        Ok(bound) if (0..=MAX_TRADE_PERCENT).contains(&bound) => Ok(Some(bound)),
        _ => Err(ApiError::bad_request("Invalid trade percent")),
    }
}

fn parse_filter_subtype(
    filter_type: Option<ReportsFilterType>,
    value: Option<&str>,
) -> Result<Option<ReportsFilterSubtype>, ApiError> {
    let Some(value) = value else {
        return Ok(None);
    };

    let parsed = match (filter_type, value) {
        (Some(ReportsFilterType::Kvk), "1") => ReportsFilterSubtype::KvkSeason1,
        (Some(ReportsFilterType::Kvk), "2") => ReportsFilterSubtype::KvkSeason2,
        (Some(ReportsFilterType::Kvk), "3") => ReportsFilterSubtype::KvkSeason3,
        (Some(ReportsFilterType::Kvk), "100") => ReportsFilterSubtype::KvkSeasonOfConquest,
        (Some(ReportsFilterType::Ark), "1") => ReportsFilterSubtype::ArkGoldenBattleground,
        (Some(ReportsFilterType::Ark), "6") => ReportsFilterSubtype::ArkSilverBattleground,
        (Some(ReportsFilterType::Ark), "3") => ReportsFilterSubtype::ArkOsirisLeague,
        (Some(ReportsFilterType::Ark), "2") => ReportsFilterSubtype::ArkPracticeMatch,
        (Some(ReportsFilterType::Ark), "5") => ReportsFilterSubtype::ArkCustomMatch,
        _ => return Err(ApiError::bad_request("Invalid subtype for type")),
    };

    Ok(Some(parsed))
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
    fn parses_receiver_governor_id_separately_from_participant() {
        let parsed = parse_reports_request(&FxHashMap::from_iter([(
            "receiver".to_string(),
            "123".to_string(),
        )]))
        .expect("valid receiver");

        assert_eq!(parsed.receiver_id, Some(123));
        assert_eq!(parsed.player_id, None);
    }

    #[test]
    fn rejects_invalid_receiver_governor_id() {
        for value in ["", "abc", "player_123", "0", "-1", "9223372036854775808"] {
            parse_reports_request(&FxHashMap::from_iter([(
                "receiver".to_string(),
                value.to_string(),
            )]))
            .expect_err("invalid receiver should be rejected");
        }
    }

    #[test]
    fn parses_filter_side() {
        let parsed = parse_reports_request(&FxHashMap::from_iter([
            ("rs".to_string(), "sender".to_string()),
            ("gs".to_string(), "none".to_string()),
        ]))
        .expect("parsed request");
        assert_eq!(parsed.rally_side, ReportsFilterSide::Sender);
    }

    #[test]
    fn rejects_non_numeric_cursor() {
        let result = parse_reports_request(&FxHashMap::from_iter([(
            "after".to_string(),
            "abc".to_string(),
        )]));
        result.expect_err("input should be rejected");
    }

    #[test]
    fn parses_open_field() {
        let parsed =
            parse_reports_request(&FxHashMap::from_iter([("of".to_string(), "1".to_string())]))
                .expect("parsed open field");
        assert!(parsed.open_field);
        assert!(!parse_reports_request(&FxHashMap::default()).expect("parsed default").open_field);
    }

    #[test]
    fn rejects_open_field_with_rally_or_garrison() {
        for (key, value) in [("rs", "sender"), ("gs", "both")] {
            parse_reports_request(&FxHashMap::from_iter([
                ("of".to_string(), "1".to_string()),
                (key.to_string(), value.to_string()),
            ]))
            .expect_err("open field with rally or garrison should be rejected");
        }
    }

    #[test]
    fn parses_either_side_commander_pair() {
        let parsed = parse_reports_request(&FxHashMap::from_iter([
            ("cpc".to_string(), "15".to_string()),
            ("csc".to_string(), "134".to_string()),
        ]))
        .expect("parsed commander pair");
        assert_eq!(parsed.commander_primary_id, Some(15));
        assert_eq!(parsed.commander_secondary_id, Some(134));
        parse_reports_request(&FxHashMap::from_iter([("cpc".to_string(), "x".to_string())]))
            .expect_err("invalid commander id should be rejected");
    }

    #[test]
    fn parses_troop_type_ids_and_trade_bounds() {
        let parsed = parse_reports_request(&FxHashMap::from_iter([
            ("cps".to_string(), "3, 15,99".to_string()),
            ("tmin".to_string(), "150".to_string()),
            ("tmax".to_string(), "400".to_string()),
        ]))
        .expect("parsed troop type and trade filters");
        assert_eq!(parsed.commander_primary_ids, vec![3, 15, 99]);
        assert_eq!(parsed.trade_min, Some(150));
        assert_eq!(parsed.trade_max, Some(400));
    }

    #[test]
    fn rejects_bad_troop_type_ids_and_trade_bounds() {
        for (key, value) in [
            ("cps", "3,,5"),
            ("cps", "0"),
            ("cps", "abc"),
            ("tmin", "-1"),
            ("tmin", "abc"),
            ("tmax", "99999999999"),
        ] {
            parse_reports_request(&FxHashMap::from_iter([(key.to_string(), value.to_string())]))
                .expect_err("invalid value should be rejected");
        }
        parse_reports_request(&FxHashMap::from_iter([
            ("tmin".to_string(), "300".to_string()),
            ("tmax".to_string(), "100".to_string()),
        ]))
        .expect_err("min above max should be rejected");
    }

    #[test]
    fn parses_time_range() {
        let parsed = parse_reports_request(&FxHashMap::from_iter([
            ("tfrom".to_string(), "1755216000".to_string()),
            ("tto".to_string(), "1755302399".to_string()),
        ]))
        .expect("parsed time range");
        assert_eq!(parsed.time_from, Some(1_755_216_000));
        assert_eq!(parsed.time_to, Some(1_755_302_399));
    }

    #[test]
    fn rejects_bad_time_range() {
        for (key, value) in [("tfrom", "-1"), ("tfrom", "abc"), ("tto", "99999999999")] {
            parse_reports_request(&FxHashMap::from_iter([(key.to_string(), value.to_string())]))
                .expect_err("invalid time should be rejected");
        }
        parse_reports_request(&FxHashMap::from_iter([
            ("tfrom".to_string(), "200".to_string()),
            ("tto".to_string(), "100".to_string()),
        ]))
        .expect_err("start after end should be rejected");
    }

    #[test]
    fn rejects_overlapping_sides() {
        let result = parse_reports_request(&FxHashMap::from_iter([
            ("rs".to_string(), "both".to_string()),
            ("gs".to_string(), "sender".to_string()),
        ]));
        result.expect_err("input should be rejected");
    }

    #[test]
    fn parses_type_scoped_subtypes() {
        let kvk = parse_reports_request(&FxHashMap::from_iter([
            ("type".to_string(), "kvk".to_string()),
            ("subtype".to_string(), "100".to_string()),
        ]))
        .expect("parsed KVK subtype");
        assert_eq!(kvk.filter_subtype, Some(ReportsFilterSubtype::KvkSeasonOfConquest));

        let ark = parse_reports_request(&FxHashMap::from_iter([
            ("type".to_string(), "ark".to_string()),
            ("subtype".to_string(), "6".to_string()),
        ]))
        .expect("parsed Ark subtype");
        assert_eq!(ark.filter_subtype, Some(ReportsFilterSubtype::ArkSilverBattleground));
    }

    #[test]
    fn rejects_subtype_without_matching_type() {
        let result = parse_reports_request(&FxHashMap::from_iter([(
            "subtype".to_string(),
            "1".to_string(),
        )]));
        result.expect_err("input should be rejected");
    }
}
