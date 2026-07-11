//! Precompute global legendary commander pairing battle summaries.

use std::collections::{BTreeMap, BTreeSet};

use core_bson::{bson_to_f64, bson_to_i64};
use drastc::{BattleRecord, DrastcModel, DrastcReferenceRanges, DrastcScore, ReferenceRange};
use futures::StreamExt;
use mongodb::{
    Collection,
    bson::{Bson, DateTime, Document, doc},
};
use rokbattles_api::db::ReportsStore;

use crate::error::JobsError;

const COMMANDERS_YAML: &str = include_str!("../../../datasets/commanders.yaml");
const BULK_WRITE_BATCH_SIZE: usize = 1_000;
const MIN_REFERENCE_RANGE_PAIRING_BATTLES: i64 = 5_000;

/// Counts from one commander pairing precompute run.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CommanderPairingsPrecomputeStats {
    pub legendary_commanders: usize,
    pub observed_pairings: usize,
    pub supported_drastc_pairings: usize,
    pub scored_drastc_pairings: usize,
    pub battle_entries_counted: i64,
    pub documents_written: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PairingKey {
    primary_commander_id: i64,
    secondary_commander_id: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct PairingRawTotals {
    total_battles: i64,
    kill_points_gained: i64,
    kill_points_lost: i64,
    trade_percentage_total: f64,
    battle_duration_total: i64,
    severely_wounded_inflicted: i64,
    severely_wounded_taken: i64,
    damage_total: i64,
    sps_total: i64,
    tps_total: i64,
    healing_total: i64,
    opponent_dead: i64,
    opponent_slightly_wounded: i64,
    sender_dead: i64,
    sender_slightly_wounded: i64,
    normalized_duration_seconds_total: f64,
    decisive_battles: i64,
    wins: i64,
    positive_trades: i64,
}

impl PairingRawTotals {
    fn to_drastc_record(self) -> BattleRecord {
        BattleRecord {
            sample_count: non_negative_i64_to_u64(self.total_battles),
            total_duration_seconds: self.normalized_duration_seconds_total,
            kill_points: self.kill_points_gained as f64,
            opponent_kill_points: self.kill_points_lost as f64,
            opponent_dead: self.opponent_dead as f64,
            opponent_severely_wounded: self.severely_wounded_inflicted as f64,
            opponent_slightly_wounded: self.opponent_slightly_wounded as f64,
            sender_dead: self.sender_dead as f64,
            sender_severely_wounded: self.severely_wounded_taken as f64,
            sender_slightly_wounded: self.sender_slightly_wounded as f64,
            sender_healing: self.healing_total as f64,
            decisive_battles: non_negative_i64_to_u64(self.decisive_battles),
            wins: non_negative_i64_to_u64(self.wins),
            positive_trades: non_negative_i64_to_u64(self.positive_trades),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct PairingSummary {
    total_battles: i64,
    kill_points_gained: i64,
    kill_points_lost: i64,
    avg_trade_percentage: f64,
    weighted_trade_percentage: f64,
    avg_battle_duration: f64,
    total_battle_duration: i64,
    severely_wounded_inflicted: i64,
    severely_wounded_taken: i64,
    dps: f64,
    sps: f64,
    tps: f64,
    hps: f64,
}

/// Refresh global legendary commander pairing summaries.
pub async fn precompute_commander_pairings_data(
    reports_store: &ReportsStore,
) -> Result<CommanderPairingsPrecomputeStats, JobsError> {
    let legendary_ids = legendary_commander_ids()?;
    let (observed, reference_ranges) =
        read_pairings_and_reference_ranges(reports_store.battle_collection(), &legendary_ids)
            .await?;
    let supported_drastc_pairings = supported_drastc_pairings(&legendary_ids);
    let drastc_scores = build_drastc_scores_from_aggregates(
        &observed,
        &supported_drastc_pairings,
        reference_ranges,
    );

    let refreshed_at = DateTime::now();
    let output = reports_store.precomputed_commander_pairings_collection();
    let all_pairings = ordered_pairing_keys(&legendary_ids);
    let mut battle_entries_counted = 0_i64;
    let mut documents = Vec::with_capacity(all_pairings.len());

    for key in all_pairings {
        let raw = observed.get(&key).copied().unwrap_or_default();
        battle_entries_counted += raw.total_battles;
        let summary = finalize_summary(raw);
        let document =
            build_precomputed_document(key, summary, drastc_scores.get(&key), refreshed_at);

        documents.push((key, document));
    }

    let documents_written = write_pairing_documents(output, documents).await?;

    Ok(CommanderPairingsPrecomputeStats {
        legendary_commanders: legendary_ids.len(),
        observed_pairings: observed.len(),
        supported_drastc_pairings: supported_drastc_pairings.len(),
        scored_drastc_pairings: drastc_scores.len(),
        battle_entries_counted,
        documents_written,
    })
}

async fn write_pairing_documents(
    output: &Collection<Document>,
    documents: Vec<(PairingKey, Document)>,
) -> Result<usize, JobsError> {
    let total_documents = documents.len();
    let mut documents_written = 0_usize;
    let mut models = Vec::with_capacity(BULK_WRITE_BATCH_SIZE);

    for (key, document) in documents {
        let mut model = output.replace_one_model(pairing_selector(key), &document)?;
        model.upsert = Some(true);
        models.push(model);

        if models.len() >= BULK_WRITE_BATCH_SIZE {
            let batch_size = models.len();
            output.client().bulk_write(models).ordered(false).await?;
            documents_written += batch_size;
            models = Vec::with_capacity(BULK_WRITE_BATCH_SIZE);
        }
    }

    if !models.is_empty() {
        let batch_size = models.len();
        output.client().bulk_write(models).ordered(false).await?;
        documents_written += batch_size;
    }

    debug_assert_eq!(documents_written, total_documents);
    Ok(documents_written)
}

fn legendary_commander_ids() -> Result<Vec<i64>, JobsError> {
    let mut ids = BTreeSet::new();
    let mut current_id = None;
    let mut current_is_legendary = false;

    for line in COMMANDERS_YAML.lines() {
        if let Some(id) = parse_top_level_commander_id(line) {
            if current_is_legendary && let Some(previous_id) = current_id {
                ids.insert(previous_id);
            }

            current_id = Some(id);
            current_is_legendary = false;
            continue;
        }

        if line.trim() == "rarity: legendary" {
            current_is_legendary = true;
        }
    }

    if current_is_legendary && let Some(previous_id) = current_id {
        ids.insert(previous_id);
    }

    if ids.is_empty() {
        return Err(JobsError::MissingLegendaryCommanders);
    }

    Ok(ids.into_iter().collect())
}

fn parse_top_level_commander_id(line: &str) -> Option<i64> {
    let rest = line.strip_prefix("  ")?;
    if rest.starts_with(' ') {
        return None;
    }

    let id = rest.strip_suffix(':')?;
    id.parse::<i64>().ok()
}

fn ordered_pairing_keys(legendary_ids: &[i64]) -> Vec<PairingKey> {
    let mut keys = Vec::with_capacity(legendary_ids.len().saturating_mul(legendary_ids.len()));

    for primary_commander_id in legendary_ids {
        for secondary_commander_id in legendary_ids {
            if primary_commander_id == secondary_commander_id {
                continue;
            }

            keys.push(PairingKey {
                primary_commander_id: *primary_commander_id,
                secondary_commander_id: *secondary_commander_id,
            });
        }
    }

    keys
}

async fn read_pairings_and_reference_ranges(
    source: &Collection<Document>,
    legendary_ids: &[i64],
) -> Result<(BTreeMap<PairingKey, PairingRawTotals>, DrastcReferenceRanges), JobsError> {
    let pipeline = build_pairings_pipeline(legendary_ids);
    let mut cursor = source.aggregate(pipeline).allow_disk_use(true).await?;

    if let Some(next) = cursor.next().await {
        let document = next?;
        return Ok(map_pairings_result_document(&document));
    }

    Ok((BTreeMap::new(), default_reference_ranges()))
}

fn build_pairings_pipeline(legendary_ids: &[i64]) -> Vec<Document> {
    let legendary_id_values = legendary_id_bson_array(legendary_ids);
    let sender_entry = conditional_entry(
        ids_match_condition(
            "$sender.commanders.primary.id",
            "$sender.commanders.secondary.id",
            &legendary_id_values,
        ),
        perspective_entry(
            "$sender.commanders.primary.id",
            "$sender.commanders.secondary.id",
            "opponents.battle_results.sender",
            "opponents.battle_results.opponent",
        ),
    );
    let opponent_entry = conditional_entry(
        ids_match_condition(
            "$opponents.commanders.primary.id",
            "$opponents.commanders.secondary.id",
            &legendary_id_values,
        ),
        perspective_entry(
            "$opponents.commanders.primary.id",
            "$opponents.commanders.secondary.id",
            "opponents.battle_results.opponent",
            "opponents.battle_results.sender",
        ),
    );

    vec![
        doc! {
            "$match": {
                "$or": [
                    {
                        "sender.commanders.primary.id": { "$in": legendary_id_values.clone() },
                        "sender.commanders.secondary.id": { "$in": legendary_id_values.clone() },
                    },
                    {
                        "opponents": {
                            "$elemMatch": {
                                "player_id": { "$gt": 0 },
                                "commanders.primary.id": { "$in": legendary_id_values.clone() },
                                "commanders.secondary.id": { "$in": legendary_id_values.clone() },
                            }
                        }
                    },
                ],
            }
        },
        doc! {
            "$set": {
                "_exclude_in_ranges": rally_garrison_report_expr(),
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
        doc! {
            "$facet": {
                "observed": observed_pairings_subpipeline(),
                "reference_ranges": reference_ranges_subpipeline(),
            }
        },
    ]
}

fn rally_garrison_report_expr() -> Document {
    doc! {
        "$or": [
            { "$in": ["$sender.rally", [Bson::Boolean(true), Bson::Int32(1), Bson::Int64(1)]] },
            { "$ne": ["$sender.alliance_building_id", Bson::Null] },
            { "$ne": ["$sender.structure_id", Bson::Null] },
            {
                "$gt": [
                    {
                        "$size": {
                            "$filter": {
                                "input": { "$ifNull": ["$opponents", []] },
                                "as": "opponent",
                                "cond": {
                                    "$or": [
                                        { "$in": ["$$opponent.rally", [Bson::Boolean(true), Bson::Int32(1), Bson::Int64(1)]] },
                                        { "$ne": ["$$opponent.alliance_building_id", Bson::Null] },
                                        { "$ne": ["$$opponent.structure_id", Bson::Null] },
                                    ]
                                },
                            }
                        }
                    },
                    0,
                ]
            },
        ]
    }
}

fn observed_pairings_subpipeline() -> Vec<Document> {
    vec![
        doc! {
            "$group": {
                "_id": {
                    "primary_commander_id": "$primary_commander_id",
                    "secondary_commander_id": "$secondary_commander_id",
                },
                "total_battles": { "$sum": 1 },
                "kill_points_gained": { "$sum": "$kill_points_gained" },
                "kill_points_lost": { "$sum": "$kill_points_lost" },
                "trade_percentage_total": { "$sum": "$trade_percentage" },
                "battle_duration_total": { "$sum": "$battle_duration" },
                "severely_wounded_inflicted": {
                    "$sum": "$severely_wounded_inflicted",
                },
                "severely_wounded_taken": { "$sum": "$severely_wounded_taken" },
                "damage_total": { "$sum": "$damage_total" },
                "sps_total": { "$sum": "$sps_total" },
                "tps_total": { "$sum": "$tps_total" },
                "healing_total": { "$sum": "$healing_total" },
                "opponent_dead": { "$sum": "$opponent_dead" },
                "opponent_slightly_wounded": { "$sum": "$opponent_slightly_wounded" },
                "sender_dead": { "$sum": "$sender_dead" },
                "sender_slightly_wounded": { "$sum": "$sender_slightly_wounded" },
                "normalized_duration_seconds_total": {
                    "$sum": "$normalized_duration_seconds",
                },
                "decisive_battles": { "$sum": "$decisive_battle" },
                "wins": { "$sum": "$win" },
                "positive_trades": { "$sum": "$positive_trade" },
            }
        },
        doc! {
            "$project": {
                "_id": 0,
                "primary_commander_id": "$_id.primary_commander_id",
                "secondary_commander_id": "$_id.secondary_commander_id",
                "total_battles": 1,
                "kill_points_gained": 1,
                "kill_points_lost": 1,
                "trade_percentage_total": 1,
                "battle_duration_total": 1,
                "severely_wounded_inflicted": 1,
                "severely_wounded_taken": 1,
                "damage_total": 1,
                "sps_total": 1,
                "tps_total": 1,
                "healing_total": 1,
                "opponent_dead": 1,
                "opponent_slightly_wounded": 1,
                "sender_dead": 1,
                "sender_slightly_wounded": 1,
                "normalized_duration_seconds_total": 1,
                "decisive_battles": 1,
                "wins": 1,
                "positive_trades": 1,
            }
        },
    ]
}

fn build_drastc_scores_from_aggregates(
    observed: &BTreeMap<PairingKey, PairingRawTotals>,
    supported_pairings: &[PairingKey],
    reference_ranges: DrastcReferenceRanges,
) -> BTreeMap<PairingKey, DrastcScore> {
    let mut scores = BTreeMap::new();

    for key in supported_pairings {
        let Some(raw) = observed.get(key) else {
            continue;
        };

        let mut model = DrastcModel::new();
        model.set_reference_ranges(reference_ranges);
        model.set_theoretical(key.primary_commander_id as u32, key.secondary_commander_id as u32);
        model.push(raw.to_drastc_record());

        if let Some(score) = model.evaluate() {
            scores.insert(*key, score);
        }
    }

    scores
}

fn supported_drastc_pairings(legendary_ids: &[i64]) -> Vec<PairingKey> {
    ordered_pairing_keys(legendary_ids)
        .into_iter()
        .filter(|key| {
            u32::try_from(key.primary_commander_id)
                .ok()
                .zip(u32::try_from(key.secondary_commander_id).ok())
                .is_some_and(|(primary, secondary)| DrastcModel::is_supported(primary, secondary))
        })
        .collect()
}

fn reference_ranges_subpipeline() -> Vec<Document> {
    vec![
        doc! {
            "$match": {
                "exclude_in_ranges": { "$ne": true },
            }
        },
        doc! {
            "$group": {
                "_id": {
                    "primary_commander_id": "$primary_commander_id",
                    "secondary_commander_id": "$secondary_commander_id",
                },
                "total_battles": { "$sum": 1 },
                "kill_points_gained": { "$sum": "$kill_points_gained" },
                "kill_points_lost": { "$sum": "$kill_points_lost" },
                "severely_wounded_inflicted": {
                    "$sum": "$severely_wounded_inflicted",
                },
                "severely_wounded_taken": { "$sum": "$severely_wounded_taken" },
                "healing_total": { "$sum": "$healing_total" },
                "opponent_dead": { "$sum": "$opponent_dead" },
                "opponent_slightly_wounded": { "$sum": "$opponent_slightly_wounded" },
                "sender_dead": { "$sum": "$sender_dead" },
                "sender_slightly_wounded": { "$sum": "$sender_slightly_wounded" },
                "normalized_duration_seconds_total": {
                    "$sum": "$normalized_duration_seconds",
                },
                "decisive_battles": { "$sum": "$decisive_battle" },
                "wins": { "$sum": "$win" },
                "positive_trades": { "$sum": "$positive_trade" },
            }
        },
        doc! {
            "$match": {
                "total_battles": { "$gte": MIN_REFERENCE_RANGE_PAIRING_BATTLES },
            }
        },
        doc! {
            "$project": {
                "damage_per_second": {
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
                "sustainability_per_second": {
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
                "consistency_rate": aggregate_consistency_rate_expr(),
                "trade_ratio": aggregate_trade_ratio_expr(),
            }
        },
        doc! {
            "$group": {
                "_id": Bson::Null,
                "samples": { "$sum": 1 },
                "damage": {
                    "$percentile": {
                        "input": "$damage_per_second",
                        "p": [0.1, 0.9],
                        "method": "approximate",
                    }
                },
                "sustainability": {
                    "$percentile": {
                        "input": "$sustainability_per_second",
                        "p": [0.1, 0.9],
                        "method": "approximate",
                    }
                },
                "consistency": {
                    "$percentile": {
                        "input": "$consistency_rate",
                        "p": [0.1, 0.9],
                        "method": "approximate",
                    }
                },
                "trade": {
                    "$percentile": {
                        "input": "$trade_ratio",
                        "p": [0.9],
                        "method": "approximate",
                    }
                },
            }
        },
    ]
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
        "exclude_in_ranges": "$_exclude_in_ranges",
        "kill_points_gained": kill_points_gained.clone(),
        "kill_points_lost": kill_points_lost.clone(),
        "trade_percentage": trade_percentage_expr(kill_points_gained, kill_points_lost),
        "battle_duration": battle_duration.clone(),
        "severely_wounded_inflicted": opponent_severely_wounded.clone(),
        "severely_wounded_taken": sender_severely_wounded.clone(),
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

fn map_aggregate_document(document: &Document) -> Option<(PairingKey, PairingRawTotals)> {
    let key = PairingKey {
        primary_commander_id: direct_i64(document, "primary_commander_id")?,
        secondary_commander_id: direct_i64(document, "secondary_commander_id")?,
    };

    Some((
        key,
        PairingRawTotals {
            total_battles: direct_i64(document, "total_battles").unwrap_or_default(),
            kill_points_gained: direct_i64(document, "kill_points_gained").unwrap_or_default(),
            kill_points_lost: direct_i64(document, "kill_points_lost").unwrap_or_default(),
            trade_percentage_total: direct_f64(document, "trade_percentage_total")
                .unwrap_or_default(),
            battle_duration_total: direct_i64(document, "battle_duration_total")
                .unwrap_or_default(),
            severely_wounded_inflicted: direct_i64(document, "severely_wounded_inflicted")
                .unwrap_or_default(),
            severely_wounded_taken: direct_i64(document, "severely_wounded_taken")
                .unwrap_or_default(),
            damage_total: direct_i64(document, "damage_total").unwrap_or_default(),
            sps_total: direct_i64(document, "sps_total").unwrap_or_default(),
            tps_total: direct_i64(document, "tps_total").unwrap_or_default(),
            healing_total: direct_i64(document, "healing_total").unwrap_or_default(),
            opponent_dead: direct_i64(document, "opponent_dead").unwrap_or_default(),
            opponent_slightly_wounded: direct_i64(document, "opponent_slightly_wounded")
                .unwrap_or_default(),
            sender_dead: direct_i64(document, "sender_dead").unwrap_or_default(),
            sender_slightly_wounded: direct_i64(document, "sender_slightly_wounded")
                .unwrap_or_default(),
            normalized_duration_seconds_total: direct_f64(
                document,
                "normalized_duration_seconds_total",
            )
            .unwrap_or_default(),
            decisive_battles: direct_i64(document, "decisive_battles").unwrap_or_default(),
            wins: direct_i64(document, "wins").unwrap_or_default(),
            positive_trades: direct_i64(document, "positive_trades").unwrap_or_default(),
        },
    ))
}

fn map_pairings_result_document(
    document: &Document,
) -> (BTreeMap<PairingKey, PairingRawTotals>, DrastcReferenceRanges) {
    let mut observed = BTreeMap::new();

    if let Some(Bson::Array(documents)) = document.get("observed") {
        for value in documents {
            let Bson::Document(document) = value else {
                continue;
            };

            if let Some((key, totals)) = map_aggregate_document(document) {
                observed.insert(key, totals);
            }
        }
    }

    let reference_ranges = document
        .get("reference_ranges")
        .and_then(|value| match value {
            Bson::Array(values) => values.first(),
            _ => None,
        })
        .and_then(|value| match value {
            Bson::Document(document) => map_reference_ranges_document(document),
            _ => None,
        })
        .unwrap_or_else(default_reference_ranges);

    (observed, reference_ranges)
}

fn map_reference_ranges_document(document: &Document) -> Option<DrastcReferenceRanges> {
    let samples = usize::try_from(direct_i64(document, "samples")?).ok()?;

    Some(DrastcReferenceRanges {
        damage: reference_range_from_percentiles(samples, document, "damage"),
        sustainability: reference_range_from_percentiles(samples, document, "sustainability"),
        trade: trade_reference_range_from_percentiles(samples, document),
        consistency: reference_range_from_percentiles(samples, document, "consistency"),
    })
}

fn trade_reference_range_from_percentiles(samples: usize, document: &Document) -> ReferenceRange {
    let Some(Bson::Array(values)) = document.get("trade") else {
        return ReferenceRange::new(0, 0.0, 0.0);
    };

    ReferenceRange::new(samples, 0.0, values.first().and_then(bson_to_f64).unwrap_or_default())
}

fn reference_range_from_percentiles(
    samples: usize,
    document: &Document,
    key: &str,
) -> ReferenceRange {
    let Some(Bson::Array(values)) = document.get(key) else {
        return ReferenceRange::new(0, 0.0, 0.0);
    };

    ReferenceRange::new(
        samples,
        values.first().and_then(bson_to_f64).unwrap_or_default(),
        values.get(1).and_then(bson_to_f64).unwrap_or_default(),
    )
}

fn default_reference_ranges() -> DrastcReferenceRanges {
    DrastcReferenceRanges {
        damage: ReferenceRange::new(0, 0.0, 0.0),
        sustainability: ReferenceRange::new(0, 0.0, 0.0),
        trade: ReferenceRange::new(0, 0.0, 0.0),
        consistency: ReferenceRange::new(0, 0.0, 0.0),
    }
}

fn finalize_summary(raw: PairingRawTotals) -> PairingSummary {
    PairingSummary {
        total_battles: raw.total_battles,
        kill_points_gained: raw.kill_points_gained,
        kill_points_lost: raw.kill_points_lost,
        avg_trade_percentage: divide(raw.trade_percentage_total, raw.total_battles as f64),
        weighted_trade_percentage: compute_trade_percentage(
            raw.kill_points_gained,
            raw.kill_points_lost,
        ),
        avg_battle_duration: divide(raw.battle_duration_total as f64, raw.total_battles as f64),
        total_battle_duration: raw.battle_duration_total,
        severely_wounded_inflicted: raw.severely_wounded_inflicted,
        severely_wounded_taken: raw.severely_wounded_taken,
        dps: rate_per_second(raw.damage_total, raw.battle_duration_total),
        sps: rate_per_second(raw.sps_total, raw.battle_duration_total),
        tps: rate_per_second(raw.tps_total, raw.battle_duration_total),
        hps: rate_per_second(raw.healing_total, raw.battle_duration_total),
    }
}

fn compute_trade_percentage(kill_points_gained: i64, kill_points_lost: i64) -> f64 {
    if kill_points_gained == kill_points_lost {
        100.0
    } else if kill_points_lost <= 0 {
        0.0
    } else {
        (kill_points_gained as f64 / kill_points_lost as f64) * 100.0
    }
}

fn divide(numerator: f64, denominator: f64) -> f64 {
    if denominator > 0.0 { numerator / denominator } else { 0.0 }
}

fn rate_per_second(total: i64, duration_millis: i64) -> f64 {
    divide(total as f64, duration_millis as f64 / 1000.0)
}

fn build_precomputed_document(
    key: PairingKey,
    summary: PairingSummary,
    drastc: Option<&DrastcScore>,
    refreshed_at: DateTime,
) -> Document {
    doc! {
        "primary_commander_id": key.primary_commander_id,
        "secondary_commander_id": key.secondary_commander_id,
        "summary": {
            "total_battles": summary.total_battles,
            "kill_points_gained": summary.kill_points_gained,
            "kill_points_lost": summary.kill_points_lost,
            "avg_trade_percentage": summary.avg_trade_percentage,
            "weighted_trade_percentage": summary.weighted_trade_percentage,
            "avg_battle_duration": summary.avg_battle_duration,
            "total_battle_duration": summary.total_battle_duration,
            "severely_wounded_inflicted": summary.severely_wounded_inflicted,
            "severely_wounded_taken": summary.severely_wounded_taken,
            "dps": summary.dps,
            "sps": summary.sps,
            "tps": summary.tps,
            "hps": summary.hps,
        },
        "drastc": drastc.map(drastc_score_document),
        "refreshed_at": refreshed_at,
    }
}

fn drastc_score_document(score: &DrastcScore) -> Document {
    doc! {
        "samples": u64_to_i64(score.samples),
        "breakdown": {
            "damage": category_score_document(score.breakdown.damage),
            "rage": category_score_document(score.breakdown.rage),
            "assist": category_score_document(score.breakdown.assist),
            "sustainability": category_score_document(score.breakdown.sustainability),
            "trade": category_score_document(score.breakdown.trade),
            "consistency": category_score_document(score.breakdown.consistency),
        },
        "overall": score.overall,
    }
}

fn category_score_document(score: drastc::CategoryScore) -> Document {
    doc! {
        "value": score.value,
        "p10": score.p10,
        "p90": score.p90,
        "score": score.score,
    }
}

fn non_negative_i64_to_u64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

fn u64_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn pairing_selector(key: PairingKey) -> Document {
    doc! {
        "primary_commander_id": key.primary_commander_id,
        "secondary_commander_id": key.secondary_commander_id,
    }
}

fn direct_i64(document: &Document, key: &str) -> Option<i64> {
    document.get(key).and_then(bson_to_i64)
}

fn direct_f64(document: &Document, key: &str) -> Option<f64> {
    document.get(key).and_then(bson_to_f64)
}

#[cfg(test)]
mod tests {
    use drastc::BattleRecord;
    use mongodb::bson::doc;

    use super::*;

    #[test]
    fn legendary_commander_ids_reads_expected_dataset_values() {
        let ids = legendary_commander_ids().expect("legendary ids");
        assert!(ids.contains(&509));
        assert!(ids.contains(&6));
        assert!(ids.contains(&179));
        assert!(ids.contains(&187));
        assert!(!ids.contains(&12));
    }

    #[test]
    fn ordered_pairing_keys_excludes_self_pairings() {
        let keys = ordered_pairing_keys(&[1, 2, 3]);
        assert_eq!(keys.len(), 6);
        assert!(!keys.contains(&PairingKey { primary_commander_id: 1, secondary_commander_id: 1 }));
    }

    #[test]
    fn supported_drastc_pairings_returns_only_model_supported_pairings() {
        let keys = supported_drastc_pairings(&[575, 579, 540]);

        assert!(
            keys.contains(&PairingKey { primary_commander_id: 579, secondary_commander_id: 575 })
        );
        assert!(
            !keys.contains(&PairingKey { primary_commander_id: 575, secondary_commander_id: 540 })
        );
    }

    #[test]
    fn reference_ranges_subpipeline_filters_rally_garrison_reports_only_in_reference_branch() {
        let pipeline = reference_ranges_subpipeline();

        assert_eq!(
            pipeline.first(),
            Some(&doc! {
                "$match": {
                    "exclude_in_ranges": { "$ne": true },
                }
            })
        );
    }

    #[test]
    fn reference_ranges_subpipeline_requires_minimum_pairing_battle_count() {
        let pipeline = reference_ranges_subpipeline();

        assert!(pipeline.iter().any(|stage| {
            stage
                .get_document("$match")
                .ok()
                .and_then(|matcher| matcher.get_document("total_battles").ok())
                .is_some_and(|total_battles| {
                    total_battles.get_i64("$gte").ok() == Some(MIN_REFERENCE_RANGE_PAIRING_BATTLES)
                })
        }));
    }

    #[test]
    fn build_pairings_pipeline_marks_rally_garrison_reports_without_filtering_observed_entries() {
        let pipeline = build_pairings_pipeline(&[509, 6]);

        assert!(pipeline.iter().any(|stage| {
            stage.get_document("$set").ok().and_then(|set| set.get("_exclude_in_ranges")).is_some()
        }));
        assert!(pipeline.iter().any(|stage| {
            stage
                .get_document("$facet")
                .ok()
                .and_then(|facet| facet.get_array("observed").ok())
                .is_some()
        }));
    }

    #[test]
    fn map_aggregate_document_maps_numeric_totals() {
        let document = doc! {
            "primary_commander_id": 509_i64,
            "secondary_commander_id": 6_i64,
            "total_battles": 2_i64,
            "kill_points_gained": 200_i64,
            "kill_points_lost": 100_i64,
            "trade_percentage_total": 300.0,
            "battle_duration_total": 10_000_i64,
            "severely_wounded_inflicted": 20_i64,
            "severely_wounded_taken": 10_i64,
            "damage_total": 50_i64,
            "sps_total": 20_i64,
            "tps_total": 10_i64,
            "healing_total": 30_i64,
            "opponent_dead": 5_i64,
            "opponent_slightly_wounded": 30_i64,
            "sender_dead": 2_i64,
            "sender_slightly_wounded": 15_i64,
            "normalized_duration_seconds_total": 10.0,
            "decisive_battles": 1_i64,
            "wins": 1_i64,
            "positive_trades": 1_i64,
        };

        let (_, totals) = map_aggregate_document(&document).expect("aggregate document");
        assert_eq!(totals.total_battles, 2);
        assert_eq!(totals.damage_total, 50);
        assert_eq!(totals.opponent_dead, 5);
        assert_eq!(totals.normalized_duration_seconds_total, 10.0);
        assert_eq!(totals.positive_trades, 1);
    }

    #[test]
    fn map_reference_ranges_document_maps_percentile_arrays() {
        let document = doc! {
            "samples": 10_i64,
            "damage": [1.0, 9.0],
            "sustainability": [-5.0, 5.0],
            "trade": [1.8],
            "consistency": [0.2, 0.8],
        };

        let ranges = map_reference_ranges_document(&document).expect("reference ranges");

        assert_eq!(ranges.damage.p10, 1.0);
        assert_eq!(ranges.damage.p90, 9.0);
        assert_eq!(ranges.damage.sample_count(), 10);
        assert_eq!(ranges.trade.p10, 0.0);
        assert_eq!(ranges.trade.p90, 1.8);
        assert_eq!(ranges.trade.sample_count(), 10);
    }

    #[test]
    fn finalize_summary_computes_averages_and_rates() {
        let summary = finalize_summary(PairingRawTotals {
            total_battles: 2,
            kill_points_gained: 200,
            kill_points_lost: 100,
            trade_percentage_total: 300.0,
            battle_duration_total: 10_000,
            severely_wounded_inflicted: 20,
            severely_wounded_taken: 10,
            damage_total: 50,
            sps_total: 20,
            tps_total: 10,
            healing_total: 30,
            ..PairingRawTotals::default()
        });

        assert_eq!(summary.avg_trade_percentage, 150.0);
        assert_eq!(summary.weighted_trade_percentage, 200.0);
        assert_eq!(summary.avg_battle_duration, 5_000.0);
        assert_eq!(summary.dps, 5.0);
        assert_eq!(summary.hps, 3.0);
    }

    #[test]
    fn build_drastc_scores_from_aggregates_scores_supported_observed_pairings() {
        let key = PairingKey { primary_commander_id: 579, secondary_commander_id: 575 };
        let observed = BTreeMap::from([(
            key,
            PairingRawTotals {
                total_battles: 2,
                kill_points_gained: 250,
                kill_points_lost: 200,
                battle_duration_total: 150_000,
                severely_wounded_inflicted: 25,
                severely_wounded_taken: 25,
                healing_total: 15,
                opponent_dead: 10,
                opponent_slightly_wounded: 90,
                sender_dead: 10,
                sender_slightly_wounded: 70,
                normalized_duration_seconds_total: 150.0,
                decisive_battles: 2,
                wins: 1,
                positive_trades: 1,
                ..PairingRawTotals::default()
            },
        )]);
        let ranges = DrastcReferenceRanges {
            damage: ReferenceRange::new(10, 0.0, 4.0),
            sustainability: ReferenceRange::new(10, -2.0, 2.0),
            trade: ReferenceRange::new(10, 0.0, 2.0),
            consistency: ReferenceRange::new(10, 0.0, 1.0),
        };

        let scores = build_drastc_scores_from_aggregates(&observed, &[key], ranges);

        let score = scores.get(&key).expect("drastc score");
        assert_eq!(score.samples, 2);
        assert_eq!(score.breakdown.rage.value, 8.0);
        assert_eq!(score.breakdown.assist.value, 14.0);
    }

    #[test]
    fn build_precomputed_document_sets_null_drastc_when_score_is_missing() {
        let document = build_precomputed_document(
            PairingKey { primary_commander_id: 1, secondary_commander_id: 2 },
            PairingSummary::default(),
            None,
            DateTime::from_millis(0),
        );

        assert!(matches!(document.get("drastc"), Some(Bson::Null)));
    }

    #[test]
    fn build_precomputed_document_embeds_drastc_score_when_present() {
        let mut model = DrastcModel::new();
        model.set_reference_ranges(DrastcReferenceRanges {
            damage: ReferenceRange::new(1, 0.0, 4.0),
            sustainability: ReferenceRange::new(1, -2.0, 2.0),
            trade: ReferenceRange::new(1, 0.0, 2.0),
            consistency: ReferenceRange::new(1, 0.0, 1.0),
        });
        model.set_theoretical(579, 575);
        model.push(BattleRecord {
            sample_count: 1,
            total_duration_seconds: 100.0,
            kill_points: 200.0,
            opponent_kill_points: 100.0,
            opponent_dead: 10.0,
            opponent_severely_wounded: 20.0,
            opponent_slightly_wounded: 70.0,
            sender_dead: 0.0,
            sender_severely_wounded: 10.0,
            sender_slightly_wounded: 30.0,
            sender_healing: 5.0,
            decisive_battles: 1,
            wins: 1,
            positive_trades: 1,
        });
        let score = model.evaluate().expect("score");

        let document = build_precomputed_document(
            PairingKey { primary_commander_id: 579, secondary_commander_id: 575 },
            PairingSummary::default(),
            Some(&score),
            DateTime::from_millis(0),
        );

        let drastc = document.get_document("drastc").expect("drastc document");
        assert_eq!(drastc.get_i64("samples").ok(), Some(1));
    }
}
