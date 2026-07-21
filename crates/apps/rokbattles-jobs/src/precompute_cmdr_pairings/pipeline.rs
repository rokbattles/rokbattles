use std::collections::BTreeSet;

use mongodb::bson::{Bson, Document, doc};

use super::model::{PairingKey, Strategy};

const MIN_REFERENCE_RANGE_PAIRING_BATTLES: i64 = 5_000;

pub(super) fn build_pairings_pipeline(legendary_ids: &[i64]) -> Vec<Document> {
    let mut pipeline = build_pairing_entries_pipeline(legendary_ids);
    pipeline.extend([
        raw_totals_group_stage(doc! {
            "primary_commander_id": "$primary_commander_id",
            "secondary_commander_id": "$secondary_commander_id",
            "strategy": "$strategy",
            "formation": "$formation",
        }),
        strategy_totals_group_stage(),
        reference_eligibility_stage(),
        reference_metric_stage(),
        reference_window_stage(),
        pairing_output_group_stage(),
        pairing_output_project_stage(),
    ]);
    pipeline
}

fn build_pairing_entries_pipeline(legendary_ids: &[i64]) -> Vec<Document> {
    let legendary_id_values = legendary_id_bson_array(legendary_ids);
    let sender_condition = ids_match_condition(
        "$sender.commanders.primary.id",
        "$sender.commanders.secondary.id",
        &legendary_id_values,
    );
    let opponent_condition = ids_match_condition(
        "$opponents.commanders.primary.id",
        "$opponents.commanders.secondary.id",
        &legendary_id_values,
    );
    let pair_filters = vec![
        Bson::Document(doc! {
            "sender.commanders.primary.id": { "$in": legendary_id_values.clone() },
            "sender.commanders.secondary.id": { "$in": legendary_id_values.clone() },
        }),
        Bson::Document(doc! {
            "opponents": {
                "$elemMatch": {
                    "player_id": { "$gt": 0 },
                    "commanders.primary.id": { "$in": legendary_id_values.clone() },
                    "commanders.secondary.id": { "$in": legendary_id_values },
                }
            }
        }),
    ];

    let sender_entry = conditional_entry(
        sender_condition,
        perspective_entry(
            "$sender.commanders.primary.id",
            "$sender.commanders.secondary.id",
            "$sender.commanders.primary.formation",
            "$_sender_strategy",
            "opponents.battle_results.sender",
            "opponents.battle_results.opponent",
        ),
    );
    let opponent_entry = conditional_entry(
        opponent_condition,
        perspective_entry(
            "$opponents.commanders.primary.id",
            "$opponents.commanders.secondary.id",
            "$opponents.commanders.primary.formation",
            opponent_strategy_expr(),
            "opponents.battle_results.opponent",
            "opponents.battle_results.sender",
        ),
    );

    build_entries_pipeline(sender_entry, opponent_entry, pair_filters)
}

pub(super) fn build_supported_pairing_entries_pipeline(
    supported_pairings: &[PairingKey],
) -> Vec<Document> {
    let sender_condition = exact_pairing_condition(
        "$sender.commanders.primary.id",
        "$sender.commanders.secondary.id",
        supported_pairings,
    );
    let opponent_condition = exact_pairing_condition(
        "$opponents.commanders.primary.id",
        "$opponents.commanders.secondary.id",
        supported_pairings,
    );
    let primary_ids = supported_pairings
        .iter()
        .map(|key| key.primary_commander_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(Bson::Int64)
        .collect::<Vec<_>>();
    let secondary_ids = supported_pairings
        .iter()
        .map(|key| key.secondary_commander_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(Bson::Int64)
        .collect::<Vec<_>>();
    let pair_filters = vec![
        Bson::Document(doc! {
            "sender.commanders.primary.id": { "$in": primary_ids.clone() },
            "sender.commanders.secondary.id": { "$in": secondary_ids.clone() },
        }),
        Bson::Document(doc! {
            "opponents": {
                "$elemMatch": {
                    "player_id": { "$gt": 0 },
                    "commanders.primary.id": { "$in": primary_ids },
                    "commanders.secondary.id": { "$in": secondary_ids },
                }
            }
        }),
    ];

    let sender_entry = conditional_entry(
        sender_condition,
        confidence_entry(
            "$sender.commanders.primary.id",
            "$sender.commanders.secondary.id",
            "$_sender_strategy",
            "$sender.player_id",
        ),
    );
    let opponent_entry = conditional_entry(
        opponent_condition,
        confidence_entry(
            "$opponents.commanders.primary.id",
            "$opponents.commanders.secondary.id",
            opponent_strategy_expr(),
            "$opponents.player_id",
        ),
    );

    build_entries_pipeline(sender_entry, opponent_entry, pair_filters)
}

fn build_entries_pipeline(
    sender_entry: Document,
    opponent_entry: Document,
    pair_filters: Vec<Bson>,
) -> Vec<Document> {
    vec![
        doc! {
            "$match": {
                "metadata.kvk": true,
                "opponents": { "$elemMatch": { "player_id": { "$gt": 0 } } },
                "$or": pair_filters,
            }
        },
        doc! {
            "$set": {
                "_sender_strategy": sender_strategy_expr(),
            }
        },
        doc! { "$unwind": "$opponents" },
        doc! { "$match": { "opponents.player_id": { "$gt": 0 } } },
        doc! {
            "$project": {
                "entries": {
                    "$concatArrays": [
                        sender_entry,
                        opponent_entry,
                    ],
                }
            }
        },
        doc! { "$unwind": "$entries" },
        doc! { "$replaceRoot": { "newRoot": "$entries" } },
    ]
}

fn confidence_entry(
    primary_expr: &'static str,
    secondary_expr: &'static str,
    strategy_expr: impl Into<Bson>,
    player_expr: &'static str,
) -> Document {
    doc! {
        "primary_commander_id": primary_expr,
        "secondary_commander_id": secondary_expr,
        "strategy": strategy_expr.into(),
        "player_id": { "$ifNull": [player_expr, 0_i64] },
    }
}

fn exact_pairing_condition(
    primary_expr: &'static str,
    secondary_expr: &'static str,
    supported_pairings: &[PairingKey],
) -> Document {
    let pairing_values = supported_pairings
        .iter()
        .map(|key| {
            Bson::Array(vec![
                Bson::Int64(key.primary_commander_id),
                Bson::Int64(key.secondary_commander_id),
            ])
        })
        .collect::<Vec<_>>();

    doc! {
        "$in": [
            [primary_expr, secondary_expr],
            pairing_values,
        ]
    }
}

fn sender_strategy_expr() -> Document {
    doc! {
        "$switch": {
            "branches": [
                { "case": sender_garrison_expr(), "then": Strategy::Garrison.as_str() },
                { "case": { "$eq": ["$sender.rally", true] }, "then": Strategy::Rally.as_str() },
                { "case": opponent_rally_or_garrison_expr(), "then": Strategy::Swarming.as_str() },
            ],
            "default": Strategy::OpenField.as_str(),
        }
    }
}

fn opponent_strategy_expr() -> Document {
    doc! {
        "$switch": {
            "branches": [
                { "case": opponent_garrison_expr(), "then": Strategy::Garrison.as_str() },
                { "case": { "$eq": ["$opponents.rally", true] }, "then": Strategy::Rally.as_str() },
                {
                    "case": {
                        "$or": [
                            sender_garrison_expr(),
                            { "$eq": ["$sender.rally", true] },
                        ]
                    },
                    "then": Strategy::Swarming.as_str(),
                },
            ],
            "default": Strategy::OpenField.as_str(),
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
            0,
        ]
    }
}

fn raw_totals_group_stage(id: Document) -> Document {
    doc! {
        "$group": {
            "_id": id,
            "total_battles": { "$sum": 1 },
            "kill_points_gained": { "$sum": "$kill_points_gained" },
            "kill_points_lost": { "$sum": "$kill_points_lost" },
            "trade_percentage_total": { "$sum": "$trade_percentage" },
            "battle_duration_total": { "$sum": "$battle_duration" },
            "severely_wounded_inflicted": { "$sum": "$severely_wounded_inflicted" },
            "severely_wounded_taken": { "$sum": "$severely_wounded_taken" },
            "power_loss_inflicted": { "$sum": "$power_loss_inflicted" },
            "power_loss_taken": { "$sum": "$power_loss_taken" },
            "atk_power_loss_inflicted": { "$sum": "$atk_power_loss_inflicted" },
            "atk_power_loss_taken": { "$sum": "$atk_power_loss_taken" },
            "skill_power_loss_inflicted": { "$sum": "$skill_power_loss_inflicted" },
            "skill_power_loss_taken": { "$sum": "$skill_power_loss_taken" },
            "damage_total": { "$sum": "$damage_total" },
            "sps_total": { "$sum": "$sps_total" },
            "tps_total": { "$sum": "$tps_total" },
            "healing_total": { "$sum": "$healing_total" },
            "opponent_dead": { "$sum": "$opponent_dead" },
            "opponent_slightly_wounded": { "$sum": "$opponent_slightly_wounded" },
            "sender_dead": { "$sum": "$sender_dead" },
            "sender_slightly_wounded": { "$sum": "$sender_slightly_wounded" },
            "normalized_duration_seconds_total": { "$sum": "$normalized_duration_seconds" },
            "decisive_battles": { "$sum": "$decisive_battle" },
            "wins": { "$sum": "$win" },
            "positive_trades": { "$sum": "$positive_trade" },
        }
    }
}

fn strategy_totals_group_stage() -> Document {
    doc! {
        "$group": {
            "_id": {
                "primary_commander_id": "$_id.primary_commander_id",
                "secondary_commander_id": "$_id.secondary_commander_id",
                "strategy": "$_id.strategy",
            },
            "total_battles": { "$sum": "$total_battles" },
            "kill_points_gained": { "$sum": "$kill_points_gained" },
            "kill_points_lost": { "$sum": "$kill_points_lost" },
            "trade_percentage_total": { "$sum": "$trade_percentage_total" },
            "battle_duration_total": { "$sum": "$battle_duration_total" },
            "severely_wounded_inflicted": { "$sum": "$severely_wounded_inflicted" },
            "severely_wounded_taken": { "$sum": "$severely_wounded_taken" },
            "power_loss_inflicted": { "$sum": "$power_loss_inflicted" },
            "power_loss_taken": { "$sum": "$power_loss_taken" },
            "atk_power_loss_inflicted": { "$sum": "$atk_power_loss_inflicted" },
            "atk_power_loss_taken": { "$sum": "$atk_power_loss_taken" },
            "skill_power_loss_inflicted": { "$sum": "$skill_power_loss_inflicted" },
            "skill_power_loss_taken": { "$sum": "$skill_power_loss_taken" },
            "damage_total": { "$sum": "$damage_total" },
            "sps_total": { "$sum": "$sps_total" },
            "tps_total": { "$sum": "$tps_total" },
            "healing_total": { "$sum": "$healing_total" },
            "opponent_dead": { "$sum": "$opponent_dead" },
            "opponent_slightly_wounded": { "$sum": "$opponent_slightly_wounded" },
            "sender_dead": { "$sum": "$sender_dead" },
            "sender_slightly_wounded": { "$sum": "$sender_slightly_wounded" },
            "normalized_duration_seconds_total": {
                "$sum": "$normalized_duration_seconds_total",
            },
            "decisive_battles": { "$sum": "$decisive_battles" },
            "wins": { "$sum": "$wins" },
            "positive_trades": { "$sum": "$positive_trades" },
            "formations": {
                "$push": {
                    "id": "$_id.formation",
                    "count": "$total_battles",
                }
            },
        }
    }
}

fn reference_eligibility_stage() -> Document {
    doc! {
        "$set": {
            "_reference_eligible": {
                "$and": [
                    { "$eq": ["$_id.strategy", Strategy::OpenField.as_str()] },
                    { "$gte": ["$total_battles", MIN_REFERENCE_RANGE_PAIRING_BATTLES] },
                ]
            }
        }
    }
}

fn reference_metric_stage() -> Document {
    doc! {
        "$set": {
            "_reference_damage": {
                "$cond": [
                    "$_reference_eligible",
                    {
                        "$divide": [
                            {
                                "$add": [
                                    "$opponent_dead",
                                    "$severely_wounded_inflicted",
                                    "$opponent_slightly_wounded",
                                ]
                            },
                            { "$max": ["$normalized_duration_seconds_total", 1.0] },
                        ]
                    },
                    Bson::Null,
                ]
            },
            "_reference_sustainability": {
                "$cond": [
                    "$_reference_eligible",
                    {
                        "$divide": [
                            {
                                "$subtract": [
                                    "$healing_total",
                                    {
                                        "$add": [
                                            "$sender_dead",
                                            "$severely_wounded_taken",
                                            "$sender_slightly_wounded",
                                        ]
                                    },
                                ]
                            },
                            { "$max": ["$normalized_duration_seconds_total", 1.0] },
                        ]
                    },
                    Bson::Null,
                ]
            },
            "_reference_consistency": {
                "$cond": [
                    "$_reference_eligible",
                    aggregate_consistency_rate_expr(),
                    Bson::Null,
                ]
            },
            "_reference_trade": {
                "$cond": [
                    "$_reference_eligible",
                    aggregate_trade_ratio_expr(),
                    Bson::Null,
                ]
            },
        }
    }
}

fn reference_window_stage() -> Document {
    let window = doc! { "documents": ["unbounded", "unbounded"] };
    doc! {
        "$setWindowFields": {
            "output": {
                "_reference_samples": {
                    "$sum": { "$cond": ["$_reference_eligible", 1, 0] },
                    "window": window.clone(),
                },
                "_reference_damage_percentiles": {
                    "$percentile": {
                        "input": "$_reference_damage",
                        "p": [0.1, 0.9],
                        "method": "approximate",
                    },
                    "window": window.clone(),
                },
                "_reference_sustainability_percentiles": {
                    "$percentile": {
                        "input": "$_reference_sustainability",
                        "p": [0.1, 0.9],
                        "method": "approximate",
                    },
                    "window": window.clone(),
                },
                "_reference_consistency_percentiles": {
                    "$percentile": {
                        "input": "$_reference_consistency",
                        "p": [0.1, 0.9],
                        "method": "approximate",
                    },
                    "window": window.clone(),
                },
                "_reference_trade_percentiles": {
                    "$percentile": {
                        "input": "$_reference_trade",
                        "p": [0.9],
                        "method": "approximate",
                    },
                    "window": window,
                },
            }
        }
    }
}

fn pairing_output_group_stage() -> Document {
    doc! {
        "$group": {
            "_id": {
                "primary_commander_id": "$_id.primary_commander_id",
                "secondary_commander_id": "$_id.secondary_commander_id",
            },
            "strategies": {
                "$push": strategy_aggregate_document(),
            },
            "drastc_observed": {
                "$max": {
                    "$cond": [
                        { "$eq": ["$_id.strategy", Strategy::OpenField.as_str()] },
                        raw_totals_aggregate_document(),
                        Bson::Null,
                    ]
                }
            },
            "reference_samples": { "$first": "$_reference_samples" },
            "reference_damage": { "$first": "$_reference_damage_percentiles" },
            "reference_sustainability": {
                "$first": "$_reference_sustainability_percentiles",
            },
            "reference_consistency": { "$first": "$_reference_consistency_percentiles" },
            "reference_trade": { "$first": "$_reference_trade_percentiles" },
        }
    }
}

fn strategy_aggregate_document() -> Document {
    let mut document = raw_totals_aggregate_document();
    document.insert("strategy", "$_id.strategy");
    document.insert("formations", "$formations");
    document
}

fn raw_totals_aggregate_document() -> Document {
    doc! {
        "total_battles": "$total_battles",
        "kill_points_gained": "$kill_points_gained",
        "kill_points_lost": "$kill_points_lost",
        "trade_percentage_total": "$trade_percentage_total",
        "battle_duration_total": "$battle_duration_total",
        "severely_wounded_inflicted": "$severely_wounded_inflicted",
        "severely_wounded_taken": "$severely_wounded_taken",
        "power_loss_inflicted": "$power_loss_inflicted",
        "power_loss_taken": "$power_loss_taken",
        "atk_power_loss_inflicted": "$atk_power_loss_inflicted",
        "atk_power_loss_taken": "$atk_power_loss_taken",
        "skill_power_loss_inflicted": "$skill_power_loss_inflicted",
        "skill_power_loss_taken": "$skill_power_loss_taken",
        "damage_total": "$damage_total",
        "sps_total": "$sps_total",
        "tps_total": "$tps_total",
        "healing_total": "$healing_total",
        "opponent_dead": "$opponent_dead",
        "opponent_slightly_wounded": "$opponent_slightly_wounded",
        "sender_dead": "$sender_dead",
        "sender_slightly_wounded": "$sender_slightly_wounded",
        "normalized_duration_seconds_total": "$normalized_duration_seconds_total",
        "decisive_battles": "$decisive_battles",
        "wins": "$wins",
        "positive_trades": "$positive_trades",
    }
}

fn pairing_output_project_stage() -> Document {
    doc! {
        "$project": {
            "_id": 0,
            "primary_commander_id": "$_id.primary_commander_id",
            "secondary_commander_id": "$_id.secondary_commander_id",
            "strategies": 1,
            "drastc_observed": 1,
            "reference_ranges": {
                "samples": "$reference_samples",
                "damage": "$reference_damage",
                "sustainability": "$reference_sustainability",
                "consistency": "$reference_consistency",
                "trade": "$reference_trade",
            },
        }
    }
}

fn aggregate_trade_ratio_expr() -> Document {
    doc! {
        "$cond": [
            { "$and": [
                { "$lte": ["$kill_points_gained", 0] },
                { "$lte": ["$kill_points_lost", 0] },
            ] },
            1.0,
            {
                "$cond": [
                    { "$lte": ["$kill_points_lost", 0] },
                    0.0,
                    { "$divide": ["$kill_points_gained", "$kill_points_lost"] },
                ]
            },
        ]
    }
}

fn aggregate_consistency_rate_expr() -> Document {
    let positive_trade_rate = doc! {
        "$divide": ["$positive_trades", "$total_battles"]
    };
    let win_rate = doc! {
        "$divide": ["$wins", "$decisive_battles"]
    };

    doc! {
        "$cond": [
            { "$gt": ["$decisive_battles", 0] },
            { "$divide": [{ "$add": [win_rate, positive_trade_rate.clone()] }, 2.0] },
            positive_trade_rate,
        ]
    }
}

fn legendary_id_bson_array(legendary_ids: &[i64]) -> Vec<Bson> {
    legendary_ids.iter().map(|id| Bson::Int64(*id)).collect()
}

fn ids_match_condition(
    primary_expr: &'static str,
    secondary_expr: &'static str,
    legendary_ids: &[Bson],
) -> Document {
    doc! {
        "$and": [
            { "$in": [primary_expr, legendary_ids.to_vec()] },
            { "$in": [secondary_expr, legendary_ids.to_vec()] },
            { "$ne": [primary_expr, secondary_expr] },
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

fn perspective_entry(
    primary_expr: &'static str,
    secondary_expr: &'static str,
    formation_expr: &'static str,
    strategy_expr: impl Into<Bson>,
    self_results_path: &'static str,
    enemy_results_path: &'static str,
) -> Document {
    let kill_points_gained = numeric_field(self_results_path, "kill_points");
    let kill_points_lost = numeric_field(enemy_results_path, "kill_points");
    let opponent_dead = numeric_field(enemy_results_path, "dead");
    let opponent_severely_wounded = numeric_field(enemy_results_path, "severely_wounded");
    let opponent_slightly_wounded = numeric_field(enemy_results_path, "slightly_wounded");
    let sender_dead = numeric_field(self_results_path, "dead");
    let sender_severely_wounded = numeric_field(self_results_path, "severely_wounded");
    let sender_slightly_wounded = numeric_field(self_results_path, "slightly_wounded");
    let battle_duration = battle_duration_expr();
    let inflicted_lethal =
        doc! { "$add": [opponent_dead.clone(), opponent_severely_wounded.clone()] };
    let received_lethal = doc! { "$add": [sender_dead.clone(), sender_severely_wounded.clone()] };

    doc! {
        "primary_commander_id": primary_expr,
        "secondary_commander_id": secondary_expr,
        "formation": { "$ifNull": [formation_expr, 0_i64] },
        "strategy": strategy_expr.into(),
        "kill_points_gained": kill_points_gained.clone(),
        "kill_points_lost": kill_points_lost.clone(),
        "trade_percentage": trade_percentage_expr(kill_points_gained, kill_points_lost),
        "battle_duration": battle_duration.clone(),
        "severely_wounded_inflicted": opponent_severely_wounded.clone(),
        "severely_wounded_taken": sender_severely_wounded.clone(),
        "power_loss_inflicted": power_loss_field(enemy_results_path, "power"),
        "power_loss_taken": power_loss_field(self_results_path, "power"),
        "atk_power_loss_inflicted": power_loss_field(enemy_results_path, "attack_power"),
        "atk_power_loss_taken": power_loss_field(self_results_path, "attack_power"),
        "skill_power_loss_inflicted": power_loss_field(enemy_results_path, "skill_power"),
        "skill_power_loss_taken": power_loss_field(self_results_path, "skill_power"),
        "damage_total": {
            "$add": [
                opponent_slightly_wounded.clone(),
                opponent_severely_wounded.clone(),
            ]
        },
        "sps_total": opponent_severely_wounded.clone(),
        "tps_total": sender_severely_wounded.clone(),
        "healing_total": numeric_field(self_results_path, "heal"),
        "opponent_dead": opponent_dead,
        "opponent_slightly_wounded": opponent_slightly_wounded,
        "sender_dead": sender_dead,
        "sender_slightly_wounded": sender_slightly_wounded,
        "normalized_duration_seconds": {
            "$cond": [
                { "$gt": [battle_duration.clone(), 0] },
                { "$divide": [battle_duration, 1000.0] },
                1.0,
            ]
        },
        "decisive_battle": {
            "$cond": [
                { "$ne": [inflicted_lethal.clone(), received_lethal.clone()] },
                1,
                0,
            ]
        },
        "win": {
            "$cond": [
                { "$gt": [inflicted_lethal, received_lethal] },
                1,
                0,
            ]
        },
        "positive_trade": {
            "$cond": [
                { "$gt": [
                    numeric_field(self_results_path, "kill_points"),
                    numeric_field(enemy_results_path, "kill_points"),
                ] },
                1,
                0,
            ]
        },
    }
}

fn numeric_field(path: &str, field: &str) -> Document {
    doc! { "$ifNull": [format!("${path}.{field}"), 0] }
}

fn power_loss_field(path: &str, field: &str) -> Document {
    doc! {
        "$max": [
            { "$multiply": [numeric_field(path, field), -1_i64] },
            0_i64,
        ]
    }
}

fn trade_percentage_expr(kill_points_gained: Document, kill_points_lost: Document) -> Document {
    doc! {
        "$cond": [
            { "$eq": [kill_points_gained.clone(), kill_points_lost.clone()] },
            100.0,
            {
                "$cond": [
                    { "$lte": [kill_points_lost.clone(), 0] },
                    0.0,
                    {
                        "$round": [
                            {
                                "$multiply": [
                                    { "$divide": [kill_points_gained, kill_points_lost] },
                                    100.0,
                                ]
                            },
                            0,
                        ]
                    },
                ]
            },
        ]
    }
}

fn battle_duration_expr() -> Document {
    doc! {
        "$max": [
            0,
            {
                "$subtract": [
                    normalize_timestamp_expr({
                        doc! {
                            "$add": [
                                numeric_path("$timeline.start_timestamp"),
                                numeric_path("$opponents.end_tick"),
                            ]
                        }
                    }),
                    normalize_timestamp_expr({
                        doc! {
                            "$add": [
                                numeric_path("$timeline.start_timestamp"),
                                numeric_path("$opponents.start_tick"),
                            ]
                        }
                    }),
                ]
            },
        ]
    }
}

fn numeric_path(path: &'static str) -> Document {
    doc! { "$ifNull": [path, 0] }
}

fn normalize_timestamp_expr(value: Document) -> Document {
    doc! {
        "$let": {
            "vars": {
                "raw": value.clone(),
                "abs": { "$abs": value },
            },
            "in": {
                "$switch": {
                    "branches": [
                        {
                            "case": { "$lt": ["$$abs", 1_000_000_000_000_f64] },
                            "then": { "$multiply": ["$$raw", 1000.0] },
                        },
                        {
                            "case": { "$gte": ["$$abs", 100_000_000_000_000_000_f64] },
                            "then": { "$divide": ["$$raw", 1_000_000.0] },
                        },
                        {
                            "case": { "$gte": ["$$abs", 100_000_000_000_000_f64] },
                            "then": { "$divide": ["$$raw", 1000.0] },
                        },
                    ],
                    "default": "$$raw",
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{Bson, doc};

    use super::*;

    #[test]
    fn reference_eligibility_requires_large_open_field_pairings() {
        assert_eq!(
            reference_eligibility_stage(),
            doc! {
                "$set": {
                    "_reference_eligible": {
                        "$and": [
                            { "$eq": ["$_id.strategy", "open_field"] },
                            { "$gte": ["$total_battles", MIN_REFERENCE_RANGE_PAIRING_BATTLES] },
                        ]
                    }
                }
            }
        );
    }

    #[test]
    fn build_pairings_pipeline_starts_with_indexable_kvk_filter() {
        let pipeline = build_pairings_pipeline(&[509, 6]);
        let matcher = pipeline
            .first()
            .and_then(|stage| stage.get_document("$match").ok())
            .expect("leading match");

        assert_eq!(matcher.get_bool("metadata.kvk"), Ok(true));
        assert!(matcher.get_document("opponents").is_ok());
        assert!(matcher.get_array("$or").is_ok());
    }

    #[test]
    fn build_pairings_pipeline_streams_pairing_documents_without_facets() {
        let pipeline = build_pairings_pipeline(&[509, 6]);
        let group_count = pipeline.iter().filter(|stage| stage.contains_key("$group")).count();

        assert_eq!(group_count, 3);
        assert!(!pipeline.iter().any(|stage| stage.contains_key("$facet")));
        assert!(pipeline.iter().any(|stage| stage.contains_key("$setWindowFields")));
    }

    #[test]
    fn exact_pairing_condition_uses_compact_tuple_membership() {
        let condition = exact_pairing_condition(
            "$primary",
            "$secondary",
            &[
                PairingKey { primary_commander_id: 595, secondary_commander_id: 596 },
                PairingKey { primary_commander_id: 509, secondary_commander_id: 6 },
            ],
        );
        let operands = condition.get_array("$in").expect("tuple membership operands");

        assert_eq!(operands.len(), 2);
        assert_eq!(
            operands.first(),
            Some(&Bson::Array(vec![
                Bson::String("$primary".to_string()),
                Bson::String("$secondary".to_string()),
            ]))
        );
        assert_eq!(
            operands.get(1),
            Some(&Bson::Array(vec![
                Bson::Array(vec![Bson::Int64(595), Bson::Int64(596)]),
                Bson::Array(vec![Bson::Int64(509), Bson::Int64(6)]),
            ]))
        );
    }

    #[test]
    fn reference_window_computes_all_four_open_field_percentile_ranges() {
        let window = reference_window_stage();
        let output = window
            .get_document("$setWindowFields")
            .and_then(|window| window.get_document("output"))
            .expect("window output");

        for field in [
            "_reference_damage_percentiles",
            "_reference_sustainability_percentiles",
            "_reference_consistency_percentiles",
            "_reference_trade_percentiles",
        ] {
            assert!(output.get_document(field).is_ok());
        }
        let group = pairing_output_group_stage();
        let condition = group
            .get_document("$group")
            .and_then(|group| group.get_document("drastc_observed"))
            .and_then(|drastc| drastc.get_document("$max"))
            .and_then(|maximum| maximum.get_array("$cond"))
            .expect("DRASTC condition");
        assert_eq!(
            condition.first(),
            Some(&Bson::Document(doc! { "$eq": ["$_id.strategy", "open_field"] }))
        );
    }

    #[test]
    fn strategy_expressions_are_mutually_ordered_like_my_pairings() {
        let sender = sender_strategy_expr();
        let sender_branches = sender
            .get_document("$switch")
            .and_then(|switch| switch.get_array("branches"))
            .expect("sender branches");
        let opponent = opponent_strategy_expr();
        let opponent_branches = opponent
            .get_document("$switch")
            .and_then(|switch| switch.get_array("branches"))
            .expect("opponent branches");

        assert_eq!(sender_branches.len(), 3);
        assert_eq!(opponent_branches.len(), 3);
        assert_eq!(
            sender.get_document("$switch").and_then(|switch| switch.get_str("default")),
            Ok("open_field")
        );
        assert_eq!(
            opponent.get_document("$switch").and_then(|switch| switch.get_str("default")),
            Ok("open_field")
        );
    }

    #[test]
    fn sender_garrison_checks_alliance_building_and_structure_ids() {
        assert_eq!(
            sender_garrison_expr(),
            doc! {
                "$or": [
                    {
                        "$ne": [
                            { "$ifNull": ["$sender.alliance_building_id", Bson::Null] },
                            Bson::Null,
                        ]
                    },
                    {
                        "$ne": [
                            { "$ifNull": ["$sender.structure_id", Bson::Null] },
                            Bson::Null,
                        ]
                    },
                ]
            }
        );
    }

    #[test]
    fn opponent_garrison_checks_alliance_building_and_structure_ids() {
        assert_eq!(
            opponent_garrison_expr(),
            doc! {
                "$or": [
                    {
                        "$ne": [
                            { "$ifNull": ["$opponents.alliance_building_id", Bson::Null] },
                            Bson::Null,
                        ]
                    },
                    {
                        "$ne": [
                            { "$ifNull": ["$opponents.structure_id", Bson::Null] },
                            Bson::Null,
                        ]
                    },
                ]
            }
        );
    }

    #[test]
    fn swarming_checks_every_opponent_rally_and_garrison_indicator() {
        assert_eq!(
            opponent_rally_or_garrison_expr(),
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
                                        {
                                            "$ne": [
                                                {
                                                    "$ifNull": [
                                                        "$$opponent.alliance_building_id",
                                                        Bson::Null,
                                                    ]
                                                },
                                                Bson::Null,
                                            ]
                                        },
                                        {
                                            "$ne": [
                                                {
                                                    "$ifNull": [
                                                        "$$opponent.structure_id",
                                                        Bson::Null,
                                                    ]
                                                },
                                                Bson::Null,
                                            ]
                                        },
                                    ]
                                },
                            }
                        }
                    },
                    0,
                ]
            }
        );
    }

    #[test]
    fn sender_strategy_prioritizes_garrison_rally_then_swarming() {
        assert_eq!(
            sender_strategy_expr(),
            doc! {
                "$switch": {
                    "branches": [
                        { "case": sender_garrison_expr(), "then": "garrison" },
                        { "case": { "$eq": ["$sender.rally", true] }, "then": "rally" },
                        { "case": opponent_rally_or_garrison_expr(), "then": "swarming" },
                    ],
                    "default": "open_field",
                }
            }
        );
    }

    #[test]
    fn opponent_strategy_uses_current_march_perspective() {
        assert_eq!(
            opponent_strategy_expr(),
            doc! {
                "$switch": {
                    "branches": [
                        { "case": opponent_garrison_expr(), "then": "garrison" },
                        { "case": { "$eq": ["$opponents.rally", true] }, "then": "rally" },
                        {
                            "case": {
                                "$or": [
                                    sender_garrison_expr(),
                                    { "$eq": ["$sender.rally", true] },
                                ]
                            },
                            "then": "swarming",
                        },
                    ],
                    "default": "open_field",
                }
            }
        );
    }

    #[test]
    fn perspective_entry_maps_missing_formation_to_zero() {
        let entry = perspective_entry(
            "$primary",
            "$secondary",
            "$formation",
            "$strategy",
            "self_results",
            "enemy_results",
        );

        assert_eq!(entry.get_document("formation"), Ok(&doc! { "$ifNull": ["$formation", 0_i64] }));
    }

    #[test]
    fn perspective_entry_orients_and_normalizes_power_losses() {
        let entry = perspective_entry(
            "$primary",
            "$secondary",
            "$formation",
            "$strategy",
            "self_results",
            "enemy_results",
        );

        assert_eq!(
            entry.get_document("power_loss_inflicted"),
            Ok(&power_loss_field("enemy_results", "power"))
        );
        assert_eq!(
            entry.get_document("power_loss_taken"),
            Ok(&power_loss_field("self_results", "power"))
        );
        assert_eq!(
            entry.get_document("atk_power_loss_inflicted"),
            Ok(&power_loss_field("enemy_results", "attack_power"))
        );
        assert_eq!(
            entry.get_document("skill_power_loss_taken"),
            Ok(&power_loss_field("self_results", "skill_power"))
        );
    }
}
