use mongodb::bson::{Bson, Document, doc};

use super::query::{
    ReceiverBattleKind, ReportsFilterSide, ReportsFilterSubtype, ReportsFilterType,
    ReportsGarrisonBuildingType, ReportsRequest,
};
use crate::db::exclude_test_reports_filter;

/// Build the MongoDB `$match` object for battle report list queries.
pub(crate) fn build_reports_match(request: &ReportsRequest) -> Document {
    let mut match_pipeline: Vec<Document> =
        vec![exclude_test_reports_filter(), doc! { "opponents.player_id": { "$gt": 0 } }];

    if let Some(before_cursor) = request.before_cursor {
        match_pipeline.push(doc! { "metadata.mail_time": { "$gt": before_cursor } });
    } else if let Some(after_cursor) = request.after_cursor {
        match_pipeline.push(doc! { "metadata.mail_time": { "$lt": after_cursor } });
    }

    if let Some(receiver_id) = request.receiver_id {
        match_pipeline.push(doc! { "metadata.mail_receiver": format!("player_{receiver_id}") });

        if let Some((_, index)) = request.receiver_combat_index() {
            match_pipeline.push(index.candidate_filter());
        }

        if let Some(kind) = request.receiver_kind {
            match_pipeline.push(build_receiver_kind_condition(receiver_id, kind));
        }
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
        rally_conditions.push(build_rally_condition("sender."));
    }

    if matches!(request.rally_side, ReportsFilterSide::Opponent | ReportsFilterSide::Both) {
        rally_conditions.push(build_player_opponent_condition(build_rally_condition("")));
    }

    append_compound_condition(&mut match_pipeline, rally_conditions);

    let mut garrison_conditions: Vec<Document> = Vec::new();

    if matches!(request.garrison_side, ReportsFilterSide::Sender | ReportsFilterSide::Both) {
        garrison_conditions
            .push(build_garrison_condition("sender.", request.garrison_building_type));
    }

    if matches!(request.garrison_side, ReportsFilterSide::Opponent | ReportsFilterSide::Both) {
        garrison_conditions.push(build_opponent_garrison_condition(request.garrison_building_type));
    }

    append_compound_condition(&mut match_pipeline, garrison_conditions);

    if request.open_field {
        match_pipeline.push(build_open_field_condition());
    }

    doc! { "$and": match_pipeline }
}

/// Builds the `of` filter for both sides of the battle.
fn build_open_field_condition() -> Document {
    doc! {
        "$nor": [
            build_rally_condition("sender."),
            build_player_opponent_condition(build_rally_condition("")),
            build_garrison_condition("sender.", None),
            build_opponent_garrison_condition(None),
        ]
    }
}

fn build_receiver_kind_condition(receiver_id: i64, kind: ReceiverBattleKind) -> Document {
    let rally = || {
        build_condition_for_receiver_side(
            receiver_id,
            build_rally_condition("sender."),
            build_rally_condition(""),
        )
    };
    let garrison = || {
        build_condition_for_receiver_side(
            receiver_id,
            build_garrison_condition("sender.", None),
            build_garrison_condition("", None),
        )
    };
    let enemy_rally_or_garrison = || build_enemy_rally_or_garrison_condition(receiver_id);

    // Combat Lab classifies a march with both flags as a garrison.
    match kind {
        ReceiverBattleKind::Garrison => garrison(),
        ReceiverBattleKind::Rally => doc! { "$and": [rally(), { "$nor": [garrison()] }] },
        ReceiverBattleKind::Swarming => doc! {
            "$and": [enemy_rally_or_garrison(), { "$nor": [rally(), garrison()] }]
        },
        ReceiverBattleKind::OpenField => doc! {
            "$nor": [rally(), garrison(), enemy_rally_or_garrison()]
        },
    }
}

fn build_enemy_rally_or_garrison_condition(receiver_id: i64) -> Document {
    doc! {
        "$or": [
            { "$and": [
                build_receiver_membership("sender.", receiver_id),
                build_player_opponent_condition(build_rally_or_garrison_condition("")),
            ] },
            { "$and": [
                build_player_opponent_condition(build_receiver_membership("", receiver_id)),
                build_rally_or_garrison_condition("sender."),
            ] },
        ]
    }
}

fn build_condition_for_receiver_side(
    receiver_id: i64,
    sender_condition: Document,
    opponent_condition: Document,
) -> Document {
    doc! {
        "$or": [
            { "$and": [build_receiver_membership("sender.", receiver_id), sender_condition] },
            build_player_opponent_condition(doc! {
                "$and": [build_receiver_membership("", receiver_id), opponent_condition]
            }),
        ]
    }
}

fn build_receiver_membership(prefix: &str, receiver_id: i64) -> Document {
    doc! {
        "$or": [
            { format!("{prefix}player_id"): receiver_id },
            { format!("{prefix}participants.player_id"): receiver_id },
        ]
    }
}

fn build_player_opponent_condition(mut condition: Document) -> Document {
    condition.insert("player_id", doc! { "$gt": 0 });
    doc! { "opponents": { "$elemMatch": condition } }
}

fn build_rally_condition(prefix: &str) -> Document {
    doc! { format!("{prefix}rally"): true }
}

fn build_rally_or_garrison_condition(prefix: &str) -> Document {
    doc! { "$or": [build_rally_condition(prefix), build_garrison_condition(prefix, None)] }
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

fn build_garrison_condition(
    prefix: &str,
    garrison_type: Option<ReportsGarrisonBuildingType>,
) -> Document {
    let building_path = format!("{prefix}alliance_building_id");

    if let Some(garrison_type) = garrison_type {
        return doc! { building_path: garrison_building_condition(garrison_type) };
    }

    doc! {
        "$or": [
            { building_path: { "$gt": 0 } },
            { format!("{prefix}structure_id"): { "$gt": 0 } },
        ]
    }
}

fn garrison_building_condition(garrison_type: ReportsGarrisonBuildingType) -> Bson {
    match garrison_type {
        ReportsGarrisonBuildingType::Flag => Bson::Int32(1),
        ReportsGarrisonBuildingType::Fortress => Bson::Int32(3),
        ReportsGarrisonBuildingType::Other => {
            Bson::Document(doc! { "$gt": 0, "$nin": [Bson::Int32(1), Bson::Int32(3)] })
        }
    }
}

fn build_opponent_garrison_condition(
    garrison_type: Option<ReportsGarrisonBuildingType>,
) -> Document {
    let mut elem_match = doc! { "player_id": { "$gt": 0 } };
    elem_match.extend(build_garrison_condition("", garrison_type));

    doc! {
        "$and": [
            build_garrison_condition("opponents.", garrison_type),
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
    use mongodb::bson::Bson;
    use rustc_hash::FxHashMap;

    use super::*;
    use crate::routes::reports::battle::query::parse_reports_request;

    #[test]
    fn open_field_excludes_rally_and_garrison_battles() {
        fn last_condition(key: &str, value: &str) -> Document {
            let request = parse_reports_request(&FxHashMap::from_iter([(
                key.to_string(),
                value.to_string(),
            )]))
            .expect("valid battle filter");

            build_reports_match(&request)
                .get_array("$and")
                .expect("match conditions")
                .last()
                .and_then(Bson::as_document)
                .expect("role condition")
                .clone()
        }

        let open_field = last_condition("of", "1");
        let excluded = open_field.get_array("$nor").expect("open field exclusions");

        let mut expected = Vec::new();
        for key in ["rs", "gs"] {
            let either_side = last_condition(key, "both");
            expected.extend_from_slice(either_side.get_array("$or").expect("either-side filter"));
        }

        assert_eq!(excluded, &expected);
    }

    #[test]
    fn receiver_garrison_matches_the_governor_and_structure_on_the_same_side() {
        assert_eq!(
            build_receiver_kind_condition(123, ReceiverBattleKind::Garrison),
            doc! {
                "$or": [
                    { "$and": [
                        { "$or": [
                            { "sender.player_id": 123_i64 },
                            { "sender.participants.player_id": 123_i64 },
                        ] },
                        { "$or": [
                            { "sender.alliance_building_id": { "$gt": 0 } },
                            { "sender.structure_id": { "$gt": 0 } },
                        ] },
                    ] },
                    { "opponents": { "$elemMatch": {
                        "player_id": { "$gt": 0 },
                        "$and": [
                            { "$or": [
                                { "player_id": 123_i64 },
                                { "participants.player_id": 123_i64 },
                            ] },
                            { "$or": [
                                { "alliance_building_id": { "$gt": 0 } },
                                { "structure_id": { "$gt": 0 } },
                            ] },
                        ],
                    } } },
                ],
            }
        );
    }

    #[test]
    fn receiver_open_field_excludes_all_own_roles_and_swarming() {
        let match_kind = |kind| build_receiver_kind_condition(123, kind);
        let rally = match_kind(ReceiverBattleKind::Rally);
        let garrison = match_kind(ReceiverBattleKind::Garrison);
        let swarming = match_kind(ReceiverBattleKind::Swarming);

        let rally_conditions = rally.get_array("$and").expect("rally conditions");
        let swarming_conditions = swarming.get_array("$and").expect("swarming conditions");

        assert_eq!(rally_conditions[1], Bson::Document(doc! { "$nor": [garrison.clone()] }));
        assert_eq!(
            swarming_conditions[1],
            Bson::Document(doc! { "$nor": [rally_conditions[0].clone(), garrison.clone()] })
        );
        assert_eq!(
            match_kind(ReceiverBattleKind::OpenField),
            doc! { "$nor": [rally_conditions[0].clone(), garrison, swarming_conditions[0].clone()] }
        );
    }

    #[test]
    fn swarming_from_the_opponent_side_checks_the_sender_for_roles() {
        let condition = build_enemy_rally_or_garrison_condition(123);
        let branches = condition.get_array("$or").expect("swarming sides");

        assert_eq!(
            branches[1],
            Bson::Document(doc! { "$and": [
                { "opponents": { "$elemMatch": {
                    "player_id": { "$gt": 0 },
                    "$or": [
                        { "player_id": 123_i64 },
                        { "participants.player_id": 123_i64 },
                    ],
                } } },
                { "$or": [
                    { "sender.rally": true },
                    { "$or": [
                        { "sender.alliance_building_id": { "$gt": 0 } },
                        { "sender.structure_id": { "$gt": 0 } },
                    ] },
                ] },
            ] })
        );
    }

    #[test]
    fn receiver_filter_selects_only_the_governors_received_reports() {
        let request = parse_reports_request(&FxHashMap::from_iter([
            ("receiver".to_string(), "123".to_string()),
            ("after".to_string(), "456".to_string()),
        ]))
        .expect("valid receiver filter");

        assert_eq!(
            build_reports_match(&request),
            doc! {
                "$and": [
                    exclude_test_reports_filter(),
                    { "opponents.player_id": { "$gt": 0 } },
                    { "metadata.mail_time": { "$lt": 456_i64 } },
                    { "metadata.mail_receiver": "player_123" },
                ]
            }
        );
    }

    #[test]
    fn public_player_filter_still_matches_either_participant() {
        let request =
            parse_reports_request(&FxHashMap::from_iter([("pid".to_string(), "123".to_string())]))
                .expect("valid player filter");

        assert_eq!(
            build_reports_match(&request),
            doc! {
                "$and": [
                    exclude_test_reports_filter(),
                    { "opponents.player_id": { "$gt": 0 } },
                    { "$or": [
                        { "sender.player_id": 123_i64 },
                        { "opponents.player_id": 123_i64 },
                    ] },
                ]
            }
        );
    }

    #[test]
    fn home_filter_matches_non_kvk_non_dungeon_non_strife_reports() {
        let request = parse_reports_request(&FxHashMap::from_iter([(
            "type".to_string(),
            "home".to_string(),
        )]))
        .expect("valid Home filter");

        let filter = build_reports_match(&request);

        assert_eq!(
            filter,
            doc! {
                "$and": [
                    exclude_test_reports_filter(),
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
        let request = parse_reports_request(&FxHashMap::from_iter([(
            "type".to_string(),
            "strife".to_string(),
        )]))
        .expect("valid Strife filter");

        let filter = build_reports_match(&request);

        assert_eq!(
            filter,
            doc! {
                "$and": [
                    exclude_test_reports_filter(),
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
            let request = parse_reports_request(&FxHashMap::from_iter([
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
            parse_reports_request(&FxHashMap::from_iter([("type".to_string(), "kvk".to_string())]))
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
        let request = parse_reports_request(&FxHashMap::from_iter([
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
        let request = parse_reports_request(&FxHashMap::from_iter([(
            "rs".to_string(),
            "sender".to_string(),
        )]))
        .expect("valid sender rally filter");

        assert!(
            match_conditions(&build_reports_match(&request))
                .contains(&Bson::Document(doc! { "sender.rally": true }))
        );
    }

    #[test]
    fn opponent_rally_filter_matches_boolean_true() {
        let request = parse_reports_request(&FxHashMap::from_iter([(
            "rs".to_string(),
            "opponent".to_string(),
        )]))
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
    fn sender_garrison_any_includes_structures_with_commander_and_rally_filters() {
        for player_id in [None, Some("87102301")] {
            let mut params = FxHashMap::from_iter([
                ("spc".to_string(), "503".to_string()),
                ("ssc".to_string(), "184".to_string()),
                ("rs".to_string(), "opponent".to_string()),
                ("gs".to_string(), "sender".to_string()),
            ]);

            if let Some(player_id) = player_id {
                params.insert("pid".to_string(), player_id.to_string());
            }

            let request = parse_reports_request(&params).expect("valid pass garrison filter");

            assert!(match_conditions(&build_reports_match(&request)).contains(&Bson::Document(
                doc! {
                    "$or": [
                        { "sender.alliance_building_id": { "$gt": 0 } },
                        { "sender.structure_id": { "$gt": 0 } },
                    ]
                }
            )));
        }
    }

    #[test]
    fn opponent_garrison_any_requires_a_human_with_a_building_or_structure() {
        let request = parse_reports_request(&FxHashMap::from_iter([(
            "gs".to_string(),
            "opponent".to_string(),
        )]))
        .expect("valid opponent garrison filter");

        assert!(match_conditions(&build_reports_match(&request)).contains(&Bson::Document(doc! {
            "$and": [
                {
                    "$or": [
                        { "opponents.alliance_building_id": { "$gt": 0 } },
                        { "opponents.structure_id": { "$gt": 0 } },
                    ]
                },
                {
                    "opponents": {
                        "$elemMatch": {
                            "player_id": { "$gt": 0 },
                            "$or": [
                                { "alliance_building_id": { "$gt": 0 } },
                                { "structure_id": { "$gt": 0 } },
                            ]
                        }
                    }
                },
            ]
        })));
    }

    #[test]
    fn specific_sender_garrison_buildings_remain_alliance_only() {
        for (building, expected) in [
            ("flag", Bson::Int32(1)),
            ("fortress", Bson::Int32(3)),
            ("other", Bson::Document(doc! { "$gt": 0, "$nin": [1, 3] })),
        ] {
            let request = parse_reports_request(&FxHashMap::from_iter([
                ("gs".to_string(), "sender".to_string()),
                ("gb".to_string(), building.to_string()),
            ]))
            .expect("valid sender building filter");

            assert!(match_conditions(&build_reports_match(&request)).contains(&Bson::Document(
                doc! {
                    "sender.alliance_building_id": expected,
                }
            )));
        }
    }

    #[test]
    fn opponent_garrison_filter_exposes_partial_index_predicate() {
        let request = parse_reports_request(&FxHashMap::from_iter([
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
        let request = parse_reports_request(&FxHashMap::from_iter([
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
