use mongodb::bson::{Bson, Document, doc};

use super::query::{
    ReportsFilterSide, ReportsFilterSubtype, ReportsFilterType, ReportsGarrisonBuildingType,
    ReportsRequest,
};
use crate::db::exclude_test_client_filter;

/// Build the MongoDB `$match` object for battle report list queries.
pub(crate) fn build_reports_match(request: &ReportsRequest) -> Document {
    let mut match_pipeline: Vec<Document> =
        vec![exclude_test_client_filter(), doc! { "opponents.player_id": { "$gt": 0 } }];

    if let Some(before_cursor) = request.before_cursor {
        match_pipeline.push(doc! { "metadata.mail_time": { "$gt": before_cursor } });
    } else if let Some(after_cursor) = request.after_cursor {
        match_pipeline.push(doc! { "metadata.mail_time": { "$lt": after_cursor } });
    }

    if let Some(player_id) = request.player_id {
        match_pipeline.push(doc! {
            "$or": [
                { "sender.player_id": player_id },
                { "opponents.player_id": player_id },
            ]
        });
    }

    if let Some(commander_id) = request.sender_primary_commander_id {
        match_pipeline.push(doc! { "sender.commanders.primary.id": commander_id });
    }

    if let Some(commander_id) = request.sender_secondary_commander_id {
        match_pipeline.push(doc! { "sender.commanders.secondary.id": commander_id });
    }

    if let Some(commander_id) = request.opponent_primary_commander_id {
        match_pipeline.push(doc! {
            "opponents": {
                "$elemMatch": {
                    "player_id": { "$gt": 0 },
                    "commanders.primary.id": commander_id,
                }
            }
        });
    }

    if let Some(commander_id) = request.opponent_secondary_commander_id {
        match_pipeline.push(doc! {
            "opponents": {
                "$elemMatch": {
                    "player_id": { "$gt": 0 },
                    "commanders.secondary.id": commander_id,
                }
            }
        });
    }

    if let Some(filter_type) = request.filter_type {
        match filter_type {
            ReportsFilterType::Kvk => {
                match_pipeline.push(doc! { "metadata.kvk": true });
            }
            ReportsFilterType::Ark => {
                match_pipeline.push(doc! { "metadata.mail_role": "dungeon" });
            }
            ReportsFilterType::Home => {
                match_pipeline.push(doc! {
                    "$and": [
                        { "metadata.kvk": false },
                        {
                            "$or": [
                                { "metadata.mail_role": { "$lt": "dungeon" } },
                                { "metadata.mail_role": { "$gt": "dungeon" } },
                            ]
                        },
                        {
                            "$or": [
                                { "sender.supreme_strife.battle_id": { "$in": [Bson::Null, Bson::String("".to_string())] } },
                                { "sender.supreme_strife.team_id": { "$in": [Bson::Null, Bson::Int32(0), Bson::Int64(0)] } },
                            ]
                        }
                    ]
                });
            }
            ReportsFilterType::Strife => {
                match_pipeline.push(doc! {
                    "$and": [
                        { "sender.supreme_strife.battle_id": { "$gt": "" } },
                        { "sender.supreme_strife.team_id": { "$gt": 0 } },
                    ]
                });
            }
        }
    }

    if let Some(filter_subtype) = request.filter_subtype {
        let condition = match filter_subtype {
            ReportsFilterSubtype::KvkSeason1 => build_server_season_condition("1"),
            ReportsFilterSubtype::KvkSeason2 => build_server_season_condition("2"),
            ReportsFilterSubtype::KvkSeason3 => build_server_season_condition("3"),
            ReportsFilterSubtype::KvkSeasonOfConquest => build_server_season_condition("100"),
            ReportsFilterSubtype::ArkGoldenBattleground => {
                build_ark_session_condition(Some("ab"), Some(""))
            }
            ReportsFilterSubtype::ArkSilverBattleground => {
                build_ark_session_condition(None, Some("SilverEgypt"))
            }
            ReportsFilterSubtype::ArkOsirisLeague => build_ark_session_condition(Some("abl"), None),
            ReportsFilterSubtype::ArkPracticeMatch => {
                build_ark_session_condition(Some("abp"), Some("gvgn"))
            }
            ReportsFilterSubtype::ArkCustomMatch => {
                build_ark_session_condition(Some("abp"), Some("DiyEgypt"))
            }
        };
        match_pipeline.push(condition);
    }

    let mut rally_conditions: Vec<Document> = Vec::new();
    if matches!(request.rally_side, ReportsFilterSide::Sender | ReportsFilterSide::Both) {
        rally_conditions.push(doc! { "sender.rally": true });
    }
    if matches!(request.rally_side, ReportsFilterSide::Opponent | ReportsFilterSide::Both) {
        rally_conditions.push(doc! {
            "opponents": {
                "$elemMatch": {
                    "player_id": { "$gt": 0 },
                    "rally": true,
                }
            }
        });
    }
    append_compound_condition(&mut match_pipeline, rally_conditions);

    let mut garrison_conditions: Vec<Document> = Vec::new();
    if matches!(request.garrison_side, ReportsFilterSide::Sender | ReportsFilterSide::Both) {
        garrison_conditions.push(build_garrison_field_condition(
            "sender.alliance_building_id",
            request.garrison_building_type,
        ));
    }
    if matches!(request.garrison_side, ReportsFilterSide::Opponent | ReportsFilterSide::Both) {
        garrison_conditions.push(build_opponent_garrison_condition(request.garrison_building_type));
    }
    append_compound_condition(&mut match_pipeline, garrison_conditions);

    doc! { "$and": match_pipeline }
}

fn build_server_season_condition(base: &str) -> Document {
    doc! {
        "sender.server_season": { "$regex": format!(r"^{base}(?:\..*)?$") }
    }
}

fn build_ark_session_condition(mode: Option<&str>, submode: Option<&str>) -> Document {
    let mut conditions = Vec::new();
    if let Some(mode) = mode {
        conditions.push(doc! {
            "sender.session": { "$regex": session_parameter_pattern("mode", mode) }
        });
    }
    if let Some(submode) = submode {
        conditions.push(doc! {
            "sender.session": { "$regex": session_parameter_pattern("submode", submode) }
        });
    }
    doc! { "$and": conditions }
}

fn session_parameter_pattern(name: &str, value: &str) -> String {
    format!(r"(^|&){name}={value}(&|$)")
}

fn append_compound_condition(target: &mut Vec<Document>, conditions: Vec<Document>) {
    if conditions.is_empty() {
        return;
    }

    if conditions.len() == 1 {
        if let Some(condition) = conditions.into_iter().next() {
            target.push(condition);
        }
        return;
    }

    target.push(doc! { "$or": conditions });
}

fn build_garrison_field_condition(
    path: &str,
    garrison_type: Option<ReportsGarrisonBuildingType>,
) -> Document {
    let mut condition = Document::new();
    condition.insert(path, garrison_building_condition(garrison_type));
    condition
}

fn garrison_building_condition(garrison_type: Option<ReportsGarrisonBuildingType>) -> Bson {
    match garrison_type {
        Some(ReportsGarrisonBuildingType::Flag) => Bson::Int32(1),
        Some(ReportsGarrisonBuildingType::Fortress) => Bson::Int32(3),
        Some(ReportsGarrisonBuildingType::Other) => {
            Bson::Document(doc! { "$gt": 0, "$nin": [Bson::Int32(1), Bson::Int32(3)] })
        }
        None => Bson::Document(doc! { "$gt": 0 }),
    }
}

fn build_opponent_garrison_condition(
    garrison_type: Option<ReportsGarrisonBuildingType>,
) -> Document {
    let mut elem_match = doc! { "player_id": { "$gt": 0 } };
    elem_match.insert("alliance_building_id", garrison_building_condition(garrison_type));

    doc! {
        "$and": [
            build_garrison_field_condition(
                "opponents.alliance_building_id",
                garrison_type,
            ),
            {
                "opponents": {
                    "$elemMatch": elem_match,
                }
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use mongodb::bson::Bson;

    use super::*;
    use crate::routes::reports::battle::query::parse_reports_request;

    #[test]
    fn home_filter_matches_non_kvk_non_dungeon_non_strife_reports() {
        let request =
            parse_reports_request(&HashMap::from([("type".to_string(), "home".to_string())]))
                .expect("valid Home filter");

        let filter = build_reports_match(&request);

        assert_eq!(
            filter,
            doc! {
                "$and": [
                    { "sender.app_id": { "$ne": 10_088_010_i64 } },
                    { "opponents.player_id": { "$gt": 0 } },
                    {
                        "$and": [
                            { "metadata.kvk": false },
                            {
                                "$or": [
                                    { "metadata.mail_role": { "$lt": "dungeon" } },
                                    { "metadata.mail_role": { "$gt": "dungeon" } },
                                ]
                            },
                            {
                                "$or": [
                                    { "sender.supreme_strife.battle_id": { "$in": [Bson::Null, Bson::String(String::new())] } },
                                    { "sender.supreme_strife.team_id": { "$in": [Bson::Null, Bson::Int32(0), Bson::Int64(0)] } },
                                ]
                            }
                        ]
                    }
                ]
            }
        );
    }

    #[test]
    fn strife_filter_matches_active_supreme_strife_reports() {
        let request =
            parse_reports_request(&HashMap::from([("type".to_string(), "strife".to_string())]))
                .expect("valid Strife filter");

        let filter = build_reports_match(&request);

        assert_eq!(
            filter,
            doc! {
                "$and": [
                    { "sender.app_id": { "$ne": 10_088_010_i64 } },
                    { "opponents.player_id": { "$gt": 0 } },
                    {
                        "$and": [
                            { "sender.supreme_strife.battle_id": { "$gt": "" } },
                            { "sender.supreme_strife.team_id": { "$gt": 0 } },
                        ]
                    }
                ]
            }
        );
    }

    #[test]
    fn kvk_subtypes_match_base_and_dot_suffixed_server_seasons() {
        for subtype in ["1", "2", "3", "100"] {
            let request = parse_reports_request(&HashMap::from([
                ("type".to_string(), "kvk".to_string()),
                ("subtype".to_string(), subtype.to_string()),
            ]))
            .expect("valid KVK subtype");

            let filter = build_reports_match(&request);

            assert!(match_conditions(&filter).contains(&Bson::Document(doc! {
                "sender.server_season": { "$regex": format!(r"^{subtype}(?:\..*)?$") }
            })));
        }
    }

    #[test]
    fn kvk_filter_without_subtype_does_not_restrict_server_season() {
        let request =
            parse_reports_request(&HashMap::from([("type".to_string(), "kvk".to_string())]))
                .expect("valid KVK filter");

        let filter = build_reports_match(&request);

        assert!(match_conditions(&filter).iter().all(|condition| {
            condition
                .as_document()
                .is_none_or(|condition| !condition.contains_key("sender.server_season"))
        }));
    }

    #[test]
    fn ark_golden_subtype_matches_session() {
        let request = parse_reports_request(&HashMap::from([
            ("type".to_string(), "ark".to_string()),
            ("subtype".to_string(), "1".to_string()),
        ]))
        .expect("valid Ark subtype");

        let filter = build_reports_match(&request);

        assert!(match_conditions(&filter).contains(&Bson::Document(doc! {
            "$and": [
                { "sender.session": { "$regex": r"(^|&)mode=ab(&|$)" } },
                { "sender.session": { "$regex": r"(^|&)submode=(&|$)" } },
            ]
        })));
    }

    #[test]
    fn ark_silver_subtype_matches_session() {
        assert_ark_session_condition(
            "6",
            doc! {
                "$and": [
                    { "sender.session": { "$regex": r"(^|&)submode=SilverEgypt(&|$)" } }
                ]
            },
        );
    }

    #[test]
    fn ark_league_subtype_matches_session() {
        assert_ark_session_condition(
            "3",
            doc! {
                "$and": [
                    { "sender.session": { "$regex": r"(^|&)mode=abl(&|$)" } }
                ]
            },
        );
    }

    #[test]
    fn ark_practice_subtype_matches_session() {
        assert_ark_session_condition(
            "2",
            doc! {
                "$and": [
                    { "sender.session": { "$regex": r"(^|&)mode=abp(&|$)" } },
                    { "sender.session": { "$regex": r"(^|&)submode=gvgn(&|$)" } },
                ]
            },
        );
    }

    #[test]
    fn ark_custom_subtype_matches_session() {
        assert_ark_session_condition(
            "5",
            doc! {
                "$and": [
                    { "sender.session": { "$regex": r"(^|&)mode=abp(&|$)" } },
                    { "sender.session": { "$regex": r"(^|&)submode=DiyEgypt(&|$)" } },
                ]
            },
        );
    }

    #[test]
    fn sender_rally_filter_matches_boolean_true() {
        let request =
            parse_reports_request(&HashMap::from([("rs".to_string(), "sender".to_string())]))
                .expect("valid sender rally filter");

        assert!(
            match_conditions(&build_reports_match(&request))
                .contains(&Bson::Document(doc! { "sender.rally": true }))
        );
    }

    #[test]
    fn opponent_rally_filter_matches_boolean_true() {
        let request =
            parse_reports_request(&HashMap::from([("rs".to_string(), "opponent".to_string())]))
                .expect("valid opponent rally filter");

        assert!(match_conditions(&build_reports_match(&request)).contains(&Bson::Document(doc! {
            "opponents": {
                "$elemMatch": {
                    "player_id": { "$gt": 0 },
                    "rally": true,
                }
            }
        })));
    }

    #[test]
    fn sender_garrison_filter_uses_positive_numeric_building_ids() {
        let request =
            parse_reports_request(&HashMap::from([("gs".to_string(), "sender".to_string())]))
                .expect("valid sender garrison filter");

        assert!(
            match_conditions(&build_reports_match(&request))
                .contains(&Bson::Document(doc! { "sender.alliance_building_id": { "$gt": 0 } },))
        );
    }

    #[test]
    fn opponent_garrison_filter_exposes_partial_index_predicate() {
        let request = parse_reports_request(&HashMap::from([
            ("gs".to_string(), "opponent".to_string()),
            ("gb".to_string(), "other".to_string()),
        ]))
        .expect("valid opponent garrison filter");

        assert!(match_conditions(&build_reports_match(&request)).contains(&Bson::Document(doc! {
            "$and": [
                {
                    "opponents.alliance_building_id": {
                        "$gt": 0,
                        "$nin": [1, 3],
                    }
                },
                {
                    "opponents": {
                        "$elemMatch": {
                            "player_id": { "$gt": 0 },
                            "alliance_building_id": {
                                "$gt": 0,
                                "$nin": [1, 3],
                            }
                        }
                    }
                },
            ]
        })));
    }

    fn assert_ark_session_condition(subtype: &str, expected: Document) {
        let request = parse_reports_request(&HashMap::from([
            ("type".to_string(), "ark".to_string()),
            ("subtype".to_string(), subtype.to_string()),
        ]))
        .expect("valid Ark subtype");

        let filter = build_reports_match(&request);

        assert!(match_conditions(&filter).contains(&Bson::Document(expected)));
    }

    fn match_conditions(filter: &Document) -> &Vec<Bson> {
        filter.get_array("$and").expect("compound match filter")
    }
}
