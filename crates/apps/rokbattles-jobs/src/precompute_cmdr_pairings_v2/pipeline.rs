use mongodb::bson::{Bson, Document, doc};

const DAY_MS: i64 = 24 * 60 * 60 * 1_000;
const PERFORMANCE_CHUNK_MS: i64 = 32 * DAY_MS;
const LOADOUT_CHUNK_MS: i64 = 126 * DAY_MS;
const WEEK_MS: i64 = 7 * DAY_MS;

pub(super) fn performance_pipeline(
    legendary_ids: &[i64],
    start_ms: i64,
    end_ms: i64,
) -> Vec<Document> {
    let mut pipeline =
        pairing_entries_pipeline(legendary_ids, start_ms, end_ms, EntryShape::Performance);
    pipeline.extend([
        scenario_and_date_stage(PERFORMANCE_CHUNK_MS),
        doc! { "$unwind": "$c" },
        doc! {
            "$group": {
                "_id": { "p": "$p", "s": "$s", "m": "$m", "d": "$d", "c": "$c" },
                "b": { "$sum": 1_i64 },
                "kg": { "$sum": "$kg" },
                "kl": { "$sum": "$kl" },
                "si": { "$sum": "$si" },
                "st": { "$sum": "$st" },
                "du": { "$sum": "$du" },
                "rd": { "$sum": { "$cond": [{ "$gt": ["$du", 0_i64] }, "$du", 1_000_i64] } },
                "da": { "$sum": "$da" },
                "h": { "$sum": "$h" },
            }
        },
        doc! {
            "$project": {
                "_id": 0, "p": "$_id.p", "s": "$_id.s", "m": "$_id.m",
                "v": ["$_id.d", "$_id.c", "$b", "$kg", "$kl", "$si", "$st", "$du", "$rd", "$da", "$h"],
            }
        },
        doc! {
            "$group": {
                "_id": { "p": "$p", "s": "$s", "m": "$m" },
                "v": { "$push": "$v" },
            }
        },
        doc! { "$project": { "_id": 0, "p": "$_id.p", "s": "$_id.s", "m": "$_id.m", "v": 1 } },
    ]);
    pipeline
}

pub(super) fn loadout_pipeline(
    legendary_ids: &[i64],
    start_ms: i64,
    end_ms: i64,
    daily_cutoff_ms: i64,
) -> Vec<Document> {
    let mut pipeline =
        pairing_entries_pipeline(legendary_ids, start_ms, end_ms, EntryShape::Loadout);
    pipeline.extend([
        doc! { "$match": { "u": { "$gt": 0_i64 } } },
        loadout_scenario_and_date_stage(daily_cutoff_ms),
        doc! { "$unwind": "$c" },
        doc! {
            "$group": {
                "_id": { "p": "$p", "s": "$s", "m": "$m", "d": "$d", "c": "$c", "u": "$u" },
                "x": {
                    "$top": {
                        "sortBy": { "t": -1 },
                        "output": {
                            "f": "$f", "e": "$e", "a": "$a",
                            "ps": "$ps", "pe": "$pe", "ss": "$ss", "se": "$se",
                        },
                    }
                },
            }
        },
        doc! {
            "$project": {
                "_id": 0, "p": "$_id.p", "s": "$_id.s", "m": "$_id.m",
                "d": "$_id.d", "c": "$_id.c",
                "v": [
                    "$_id.d", "$_id.c", "$_id.u", "$x.f", "$x.e", "$x.a",
                    skill_build_expr("$x.ps"), "$x.pe", skill_build_expr("$x.ss"), "$x.se",
                ],
            }
        },
        doc! {
            "$group": {
                "_id": { "p": "$p", "s": "$s", "m": "$m", "d": "$d", "c": "$c" },
                "v": { "$push": "$v" },
            }
        },
        doc! {
            "$project": {
                "_id": 0, "p": "$_id.p", "s": "$_id.s", "m": "$_id.m",
                "d": "$_id.d", "c": "$_id.c", "v": 1,
            }
        },
    ]);
    pipeline
}

#[derive(Clone, Copy)]
enum EntryShape {
    Performance,
    Loadout,
}

fn pairing_entries_pipeline(
    legendary_ids: &[i64],
    start_ms: i64,
    end_ms: i64,
    shape: EntryShape,
) -> Vec<Document> {
    let ids = legendary_ids.iter().copied().map(Bson::Int64).collect::<Vec<_>>();
    let sender_condition = commander_condition(
        "$sender.commanders.primary.id",
        "$sender.commanders.secondary.id",
        &ids,
    );
    let opponent_condition = commander_condition(
        "$opponents.commanders.primary.id",
        "$opponents.commanders.secondary.id",
        &ids,
    );
    let sender = conditional_entry(
        sender_condition,
        perspective_entry(
            "$sender.commanders.primary.id",
            "$sender.commanders.secondary.id",
            "$sender.player_id",
            "$sender.commanders.primary",
            "$sender.commanders.secondary",
            "$_senderScenario",
            "opponents.battle_results.sender",
            "opponents.battle_results.opponent",
            shape,
        ),
    );
    let opponent = conditional_entry(
        opponent_condition,
        perspective_entry(
            "$opponents.commanders.primary.id",
            "$opponents.commanders.secondary.id",
            "$opponents.player_id",
            "$opponents.commanders.primary",
            "$opponents.commanders.secondary",
            opponent_scenario_expr(),
            "opponents.battle_results.opponent",
            "opponents.battle_results.sender",
            shape,
        ),
    );

    vec![
        doc! {
            "$match": {
                "metadata.kvk": true,
                "metadata.mail_time": {
                    "$gte": start_ms.saturating_mul(1_000),
                    "$lt": end_ms.saturating_mul(1_000),
                },
                "opponents": { "$elemMatch": { "player_id": { "$gt": 0_i64 } } },
                "$or": [
                    {
                        "sender.commanders.primary.id": { "$in": ids.clone() },
                        "sender.commanders.secondary.id": { "$in": ids.clone() },
                    },
                    {
                        "opponents": {
                            "$elemMatch": {
                                "player_id": { "$gt": 0_i64 },
                                "commanders.primary.id": { "$in": ids.clone() },
                                "commanders.secondary.id": { "$in": ids },
                            }
                        }
                    },
                ],
            }
        },
        doc! { "$set": { "_senderScenario": sender_scenario_expr() } },
        doc! { "$unwind": "$opponents" },
        doc! { "$match": { "opponents.player_id": { "$gt": 0_i64 } } },
        doc! { "$project": { "x": { "$concatArrays": [sender, opponent] } } },
        doc! { "$unwind": "$x" },
        doc! { "$replaceRoot": { "newRoot": "$x" } },
        doc! { "$match": { "t": { "$gte": start_ms, "$lt": end_ms } } },
    ]
}

fn scenario_and_date_stage(chunk_ms: i64) -> Document {
    doc! {
        "$set": {
            "c": [0_i64, "$c"],
            "d": { "$subtract": ["$t", { "$mod": ["$t", DAY_MS] }] },
            "m": { "$subtract": ["$t", { "$mod": ["$t", chunk_ms] }] },
        }
    }
}

fn loadout_scenario_and_date_stage(daily_cutoff_ms: i64) -> Document {
    doc! {
        "$set": {
            "c": [0_i64, "$c"],
            "d": {
                "$cond": [
                    { "$gte": ["$t", daily_cutoff_ms] },
                    { "$subtract": ["$t", { "$mod": ["$t", DAY_MS] }] },
                    { "$subtract": ["$t", { "$mod": ["$t", WEEK_MS] }] },
                ]
            },
            "m": { "$subtract": ["$t", { "$mod": ["$t", LOADOUT_CHUNK_MS] }] },
        }
    }
}

fn commander_condition(primary: &str, secondary: &str, ids: &[Bson]) -> Document {
    doc! {
        "$and": [
            { "$in": [primary, ids.to_vec()] },
            { "$in": [secondary, ids.to_vec()] },
            { "$ne": [primary, secondary] },
        ]
    }
}

fn conditional_entry(condition: Document, entry: Document) -> Document {
    doc! {
        "$cond": [
            condition,
            Bson::Array(vec![Bson::Document(entry)]),
            Bson::Array(Vec::new()),
        ]
    }
}

#[allow(clippy::too_many_arguments)]
fn perspective_entry(
    primary: &'static str,
    secondary: &'static str,
    player: &'static str,
    primary_path: &'static str,
    secondary_path: &'static str,
    scenario: impl Into<Bson>,
    own_results: &'static str,
    enemy_results: &'static str,
    shape: EntryShape,
) -> Document {
    let mut entry = doc! {
        "p": primary,
        "s": secondary,
        "t": normalize_timestamp_expr(doc! { "$ifNull": ["$metadata.mail_time", 0_i64] }),
        "u": { "$ifNull": [player, 0_i64] },
        "c": scenario.into(),
    };

    match shape {
        EntryShape::Performance => {
            let inflicted = numeric_field(enemy_results, "severely_wounded");
            entry.extend(doc! {
                "kg": numeric_field(own_results, "kill_points"),
                "kl": numeric_field(enemy_results, "kill_points"),
                "si": inflicted.clone(),
                "st": numeric_field(own_results, "severely_wounded"),
                "du": battle_duration_expr(),
                "da": { "$add": [numeric_field(enemy_results, "slightly_wounded"), inflicted] },
                "h": numeric_field(own_results, "heal"),
            });
        }
        EntryShape::Loadout => {
            entry.extend(doc! {
                "f": { "$ifNull": [format!("{primary_path}.formation"), 0_i64] },
                "e": { "$ifNull": [format!("{primary_path}.equipment"), Bson::Null] },
                "a": { "$ifNull": [format!("{primary_path}.armaments"), []] },
                "ps": { "$ifNull": [format!("{primary_path}.skills"), []] },
                "pe": { "$ifNull": [format!("{primary_path}.awakened"), Bson::Null] },
                "ss": { "$ifNull": [format!("{secondary_path}.skills"), []] },
                "se": { "$ifNull": [format!("{secondary_path}.awakened"), Bson::Null] },
            });
        }
    }
    entry
}

fn skill_build_expr(path: &str) -> Document {
    doc! {
        "$let": {
            "vars": {
                "allSkills": { "$ifNull": [path, []] },
            },
            "in": {
                "$let": {
                    "vars": { "skills": { "$slice": ["$$allSkills", 4_i64] } },
                    "in": {
                        "$cond": [
                            {
                                "$and": [
                                    { "$in": [{ "$size": "$$allSkills" }, [4_i64, 5_i64]] },
                                    {
                                        "$allElementsTrue": {
                                            "$map": {
                                                "input": "$$skills",
                                                "as": "skill",
                                                "in": {
                                                    "$and": [
                                                        { "$gt": ["$$skill.id", 0_i64] },
                                                        { "$gte": ["$$skill.level", 1_i64] },
                                                        { "$lte": ["$$skill.level", 5_i64] },
                                                    ]
                                                },
                                            }
                                        }
                                    },
                                    {
                                        "$eq": [
                                            {
                                                "$size": {
                                                    "$setUnion": [
                                                        {
                                                            "$map": {
                                                                "input": "$$skills",
                                                                "as": "skill",
                                                                "in": "$$skill.id",
                                                            }
                                                        },
                                                        [],
                                                    ]
                                                }
                                            },
                                            4_i64,
                                        ]
                                    },
                                    {
                                        "$or": [
                                            { "$eq": [{ "$size": "$$allSkills" }, 4_i64] },
                                            {
                                                "$eq": [
                                                    { "$arrayElemAt": ["$$allSkills.level", 4_i64] },
                                                    1_i64,
                                                ]
                                            },
                                        ]
                                    },
                                ]
                            },
                            {
                                "$reduce": {
                                    "input": "$$skills",
                                    "initialValue": 0_i64,
                                    "in": {
                                        "$add": [
                                            { "$multiply": ["$$value", 10_i64] },
                                            "$$this.level",
                                        ]
                                    },
                                }
                            },
                            Bson::Null,
                        ]
                    }
                }
            }
        }
    }
}

fn numeric_field(path: &str, field: &str) -> Document {
    doc! { "$toLong": { "$ifNull": [format!("${path}.{field}"), 0_i64] } }
}

fn sender_scenario_expr() -> Document {
    doc! {
        "$switch": {
            "branches": [
                { "case": sender_garrison_expr(), "then": 4_i64 },
                { "case": { "$eq": ["$sender.rally", true] }, "then": 3_i64 },
                { "case": opponent_rally_or_garrison_expr(), "then": 2_i64 },
            ],
            "default": 1_i64,
        }
    }
}

fn opponent_scenario_expr() -> Document {
    doc! {
        "$switch": {
            "branches": [
                { "case": opponent_garrison_expr(), "then": 4_i64 },
                { "case": { "$eq": ["$opponents.rally", true] }, "then": 3_i64 },
                {
                    "case": { "$or": [sender_garrison_expr(), { "$eq": ["$sender.rally", true] }] },
                    "then": 2_i64,
                },
            ],
            "default": 1_i64,
        }
    }
}

fn sender_garrison_expr() -> Document {
    doc! {
        "$or": [
            { "$ne": [{ "$ifNull": ["$sender.alliance_building_id", Bson::Null] }, Bson::Null] },
            { "$ne": [{ "$ifNull": ["$sender.structure_id", Bson::Null] }, Bson::Null] },
        ]
    }
}

fn opponent_garrison_expr() -> Document {
    doc! {
        "$or": [
            { "$ne": [{ "$ifNull": ["$opponents.alliance_building_id", Bson::Null] }, Bson::Null] },
            { "$ne": [{ "$ifNull": ["$opponents.structure_id", Bson::Null] }, Bson::Null] },
        ]
    }
}

fn opponent_rally_or_garrison_expr() -> Document {
    doc! {
        "$gt": [
            {
                "$size": {
                    "$filter": {
                        "input": { "$ifNull": ["$opponents", []] },
                        "as": "opponent",
                        "cond": {
                            "$or": [
                                { "$eq": ["$$opponent.rally", true] },
                                { "$ne": [{ "$ifNull": ["$$opponent.alliance_building_id", Bson::Null] }, Bson::Null] },
                                { "$ne": [{ "$ifNull": ["$$opponent.structure_id", Bson::Null] }, Bson::Null] },
                            ]
                        },
                    }
                }
            },
            0_i64,
        ]
    }
}

fn battle_duration_expr() -> Document {
    doc! {
        "$toLong": {
            "$max": [
                0_i64,
                {
                    "$subtract": [
                        normalize_timestamp_expr(doc! {
                            "$add": [
                                { "$ifNull": ["$timeline.start_timestamp", 0_i64] },
                                { "$ifNull": ["$opponents.end_tick", 0_i64] },
                            ]
                        }),
                        normalize_timestamp_expr(doc! {
                            "$add": [
                                { "$ifNull": ["$timeline.start_timestamp", 0_i64] },
                                { "$ifNull": ["$opponents.start_tick", 0_i64] },
                            ]
                        }),
                    ]
                },
            ]
        }
    }
}

fn normalize_timestamp_expr(value: Document) -> Document {
    doc! {
        "$toLong": {
            "$let": {
                "vars": { "raw": value.clone(), "abs": { "$abs": value } },
                "in": {
                    "$switch": {
                        "branches": [
                            {
                                "case": { "$lt": ["$$abs", 1_000_000_000_000_f64] },
                                "then": { "$multiply": ["$$raw", 1_000.0] },
                            },
                            {
                                "case": { "$gte": ["$$abs", 100_000_000_000_000_000_f64] },
                                "then": { "$divide": ["$$raw", 1_000_000.0] },
                            },
                            {
                                "case": { "$gte": ["$$abs", 100_000_000_000_000_f64] },
                                "then": { "$divide": ["$$raw", 1_000.0] },
                            },
                        ],
                        "default": "$$raw",
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn performance_pipeline_groups_once_by_day_instead_of_expanding_time_ranges() {
        let pipeline = performance_pipeline(&[1, 2], 0, 100);
        let rendered = format!("{pipeline:?}");

        assert!(!rendered.contains("range"));
        assert_eq!(rendered.matches("$unwind").count(), 3);
        assert!(!rendered.contains("$dateTrunc"));
        assert!(!rendered.contains("$sort"));
        assert!(rendered.contains("\"m\""));
        assert!(rendered.contains("\"d\""));
    }

    #[test]
    fn loadout_pipeline_compacts_skill_objects_after_selecting_latest_snapshots() {
        let pipeline = loadout_pipeline(&[1, 2], 0, 100, 50);
        let rendered = format!("{pipeline:?}");
        let top = rendered.find("$top").expect("latest snapshot selection");
        let skill_compaction = rendered.find("$slice").expect("skill compaction");

        assert!(skill_compaction > top);
        assert!(!rendered.contains("$sortArray"));
        assert!(rendered.contains("[Int64(4), Int64(5)]"));
        assert!(rendered.contains("$$allSkills.level"));
        assert!(rendered.contains("$$this.level"));
    }
}
