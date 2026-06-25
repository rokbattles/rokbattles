//! Precompute global legendary commander pairing battle summaries.

use std::collections::{BTreeMap, BTreeSet};

use futures::StreamExt;
use mongodb::{
    Collection,
    bson::{Bson, DateTime, Document, doc},
};
use rokbattles_api::db::ReportsStore;

use crate::error::JobsError;

const COMMANDERS_YAML: &str = include_str!("../../../datasets/commanders.yaml");
const BULK_WRITE_BATCH_SIZE: usize = 1_000;

/// Counts from one commander pairing precompute run.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CommanderPairingsPrecomputeStats {
    pub legendary_commanders: usize,
    pub observed_pairings: usize,
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
    let observed =
        read_observed_pairings(reports_store.battle_collection(), &legendary_ids).await?;

    let refreshed_at = DateTime::now();
    let output = reports_store.precomputed_commander_pairings_collection();
    let all_pairings = ordered_pairing_keys(&legendary_ids);
    let mut battle_entries_counted = 0_i64;
    let mut documents = Vec::with_capacity(all_pairings.len());

    for key in all_pairings {
        let raw = observed.get(&key).copied().unwrap_or_default();
        battle_entries_counted += raw.total_battles;
        let summary = finalize_summary(raw);
        let document = build_precomputed_document(key, summary, refreshed_at);

        documents.push((key, document));
    }

    let documents_written = write_pairing_documents(output, documents).await?;

    Ok(CommanderPairingsPrecomputeStats {
        legendary_commanders: legendary_ids.len(),
        observed_pairings: observed.len(),
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

async fn read_observed_pairings(
    source: &Collection<Document>,
    legendary_ids: &[i64],
) -> Result<BTreeMap<PairingKey, PairingRawTotals>, JobsError> {
    let pipeline = build_pairings_pipeline(legendary_ids);
    let mut cursor = source.aggregate(pipeline).allow_disk_use(true).await?;
    let mut observed = BTreeMap::new();

    while let Some(next) = cursor.next().await {
        let document = next?;
        if let Some((key, totals)) = map_aggregate_document(&document) {
            observed.insert(key, totals);
        }
    }

    Ok(observed)
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
        doc! {
            "$group": {
                "_id": {
                    "primary_commander_id": "$entries.primary_commander_id",
                    "secondary_commander_id": "$entries.secondary_commander_id",
                },
                "total_battles": { "$sum": 1 },
                "kill_points_gained": { "$sum": "$entries.kill_points_gained" },
                "kill_points_lost": { "$sum": "$entries.kill_points_lost" },
                "trade_percentage_total": { "$sum": "$entries.trade_percentage" },
                "battle_duration_total": { "$sum": "$entries.battle_duration" },
                "severely_wounded_inflicted": {
                    "$sum": "$entries.severely_wounded_inflicted",
                },
                "severely_wounded_taken": { "$sum": "$entries.severely_wounded_taken" },
                "damage_total": { "$sum": "$entries.damage_total" },
                "sps_total": { "$sum": "$entries.sps_total" },
                "tps_total": { "$sum": "$entries.tps_total" },
                "healing_total": { "$sum": "$entries.healing_total" },
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
            }
        },
    ]
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
    let severely_wounded_inflicted = numeric_field(enemy_results_path, "severely_wounded");
    let severely_wounded_taken = numeric_field(self_results_path, "severely_wounded");

    doc! {
        "primary_commander_id": primary_expr,
        "secondary_commander_id": secondary_expr,
        "kill_points_gained": kill_points_gained.clone(),
        "kill_points_lost": kill_points_lost.clone(),
        "trade_percentage": trade_percentage_expr(kill_points_gained, kill_points_lost),
        "battle_duration": battle_duration_expr(),
        "severely_wounded_inflicted": severely_wounded_inflicted.clone(),
        "severely_wounded_taken": severely_wounded_taken.clone(),
        "damage_total": {
            "$add": [
                numeric_field(enemy_results_path, "slightly_wounded"),
                severely_wounded_inflicted.clone(),
            ]
        },
        "sps_total": severely_wounded_inflicted,
        "tps_total": severely_wounded_taken,
        "healing_total": numeric_field(self_results_path, "heal"),
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
        },
    ))
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
        "refreshed_at": refreshed_at,
    }
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

fn bson_to_i64(value: &Bson) -> Option<i64> {
    match value {
        Bson::Int32(value) => Some(i64::from(*value)),
        Bson::Int64(value) => Some(*value),
        Bson::Double(value) if value.is_finite() => Some(*value as i64),
        _ => None,
    }
}

fn bson_to_f64(value: &Bson) -> Option<f64> {
    match value {
        Bson::Int32(value) => Some(f64::from(*value)),
        Bson::Int64(value) => Some(*value as f64),
        Bson::Double(value) if value.is_finite() => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
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
        };

        let (_, totals) = map_aggregate_document(&document).expect("aggregate document");
        assert_eq!(totals.total_battles, 2);
        assert_eq!(totals.damage_total, 50);
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
        });

        assert_eq!(summary.avg_trade_percentage, 150.0);
        assert_eq!(summary.weighted_trade_percentage, 200.0);
        assert_eq!(summary.avg_battle_duration, 5_000.0);
        assert_eq!(summary.dps, 5.0);
        assert_eq!(summary.hps, 3.0);
    }
}
