//! Precompute global legendary commander pairing battle summaries.

mod confidence;
mod mapper;
mod model;
mod output;
mod pipeline;
mod scoring;

use std::collections::{BTreeMap, BTreeSet};

use drastc::{DrastcConfidence, DrastcScore};
use mongodb::{
    Collection,
    bson::{DateTime, Document, doc},
};
use rokbattles_api::db::ReportsStore;

use self::{
    confidence::read_pairing_confidences,
    mapper::read_pairings_and_reference_ranges,
    model::{PairingKey, PairingStrategies},
    output::build_precomputed_document,
    scoring::{build_drastc_scores_from_aggregates, supported_drastc_pairings},
};
use crate::error::JobsError;

const COMMANDERS_YAML: &str = include_str!("../../../../../datasets/commanders.yaml");
const BULK_WRITE_BATCH_SIZE: usize = 1_000;

/// Counts from one commander pairing precompute run.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CommanderPairingsPrecomputeStats {
    pub legendary_commanders: usize,
    pub observed_pairings: usize,
    pub supported_drastc_pairings: usize,
    pub scored_drastc_pairings: usize,
    pub confidence_scored_pairings: usize,
    pub battle_entries_counted: i64,
    pub documents_written: usize,
}

/// Refresh global legendary commander pairing summaries.
pub async fn precompute_commander_pairings_data(
    reports_store: &ReportsStore,
) -> Result<CommanderPairingsPrecomputeStats, JobsError> {
    let legendary_ids = legendary_commander_ids()?;
    let aggregation =
        read_pairings_and_reference_ranges(reports_store.battle_collection(), &legendary_ids)
            .await?;
    let supported_drastc_pairings = supported_drastc_pairings(&legendary_ids);
    let drastc_scores = build_drastc_scores_from_aggregates(
        &aggregation.drastc_observed,
        &supported_drastc_pairings,
        aggregation.reference_ranges,
    );
    let confidences =
        read_pairing_confidences(reports_store.battle_collection(), &supported_drastc_pairings)
            .await?;

    let refreshed_at = DateTime::now();
    let all_pairings = ordered_pairing_keys(&legendary_ids);
    let battle_entries_counted = all_pairings
        .iter()
        .filter_map(|key| aggregation.strategies.get(key))
        .map(|strategies| strategies.all().totals.total_battles)
        .sum();
    let documents = build_pairing_documents(
        &all_pairings,
        &aggregation.strategies,
        &drastc_scores,
        &confidences,
        refreshed_at,
    );
    let documents_written = write_pairing_documents(
        reports_store.precomputed_commander_pairings_collection(),
        documents,
    )
    .await?;

    Ok(CommanderPairingsPrecomputeStats {
        legendary_commanders: legendary_ids.len(),
        observed_pairings: aggregation.strategies.len(),
        supported_drastc_pairings: supported_drastc_pairings.len(),
        scored_drastc_pairings: drastc_scores.len(),
        confidence_scored_pairings: confidences.len(),
        battle_entries_counted,
        documents_written,
    })
}

fn build_pairing_documents(
    pairings: &[PairingKey],
    strategies_by_pairing: &BTreeMap<PairingKey, PairingStrategies>,
    drastc_scores: &BTreeMap<PairingKey, DrastcScore>,
    confidences: &BTreeMap<PairingKey, DrastcConfidence>,
    refreshed_at: DateTime,
) -> Vec<(PairingKey, Document)> {
    let empty_strategies = PairingStrategies::default();

    pairings
        .iter()
        .map(|key| {
            let strategies = strategies_by_pairing.get(key).unwrap_or(&empty_strategies);
            let drastc = drastc_scores.get(key).zip(confidences.get(key));
            let document = build_precomputed_document(*key, strategies, drastc, refreshed_at);
            (*key, document)
        })
        .collect()
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

fn pairing_selector(key: PairingKey) -> Document {
    doc! {
        "primary_commander_id": key.primary_commander_id,
        "secondary_commander_id": key.secondary_commander_id,
    }
}

#[cfg(test)]
mod tests {
    use model::{PairingRawTotals, Strategy, StrategyRawTotals};

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
    fn build_pairing_documents_includes_pairings_without_observed_data() {
        let observed_key = PairingKey { primary_commander_id: 1, secondary_commander_id: 2 };
        let unobserved_key = PairingKey { primary_commander_id: 3, secondary_commander_id: 2 };
        let observed_strategies = PairingStrategies {
            values: BTreeMap::from([(
                Strategy::OpenField,
                StrategyRawTotals {
                    totals: PairingRawTotals { total_battles: 4, ..Default::default() },
                    ..Default::default()
                },
            )]),
        };
        let pairings = ordered_pairing_keys(&[1, 2, 3]);
        let documents = build_pairing_documents(
            &pairings,
            &BTreeMap::from([(observed_key, observed_strategies)]),
            &BTreeMap::new(),
            &BTreeMap::new(),
            DateTime::from_millis(0),
        );

        assert_eq!(documents.len(), 6);
        assert_eq!(
            documents
                .iter()
                .find(|(key, _)| *key == observed_key)
                .and_then(|(_, document)| document.get_document("summary").ok())
                .and_then(|summary| summary.get_i64("total_battles").ok()),
            Some(4)
        );
        assert_eq!(
            documents
                .iter()
                .find(|(key, _)| *key == unobserved_key)
                .and_then(|(_, document)| document.get_document("summary").ok())
                .and_then(|summary| summary.get_i64("total_battles").ok()),
            Some(0)
        );
    }
}
