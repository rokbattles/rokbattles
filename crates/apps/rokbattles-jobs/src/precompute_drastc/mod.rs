//! Precompute DRASTC scores for legendary open-field commander pairings.

mod confidence;
mod mapper;
mod model;
mod output;
mod pipeline;
mod scoring;

use mongodb::{
    Collection,
    bson::{DateTime, Document, doc},
};
use rokbattles_api::db::ReportsStore;

use self::{
    confidence::read_pairing_confidences,
    mapper::read_drastc_aggregation,
    model::PairingKey,
    output::build_drastc_document,
    scoring::{build_drastc_scores_from_aggregates, supported_drastc_pairings},
};
use crate::{commander_catalog::legendary_commander_ids, error::JobsError};

const BULK_WRITE_BATCH_SIZE: usize = 1_000;
const RANKING_WINDOW_DAYS: i64 = 365;
const MILLIS_PER_DAY: i64 = 24 * 60 * 60 * 1_000;

/// Counts from one DRASTC precompute run.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DrastcPrecomputeStats {
    pub legendary_commanders: usize,
    pub observed_pairings: usize,
    pub supported_pairings: usize,
    pub scored_pairings: usize,
    pub confidence_scored_pairings: usize,
    pub documents_written: usize,
    pub documents_stored: u64,
}

/// Refresh the materialized DRASTC scores used by Combat Lab v2.
pub async fn precompute_drastc_data(
    reports_store: &ReportsStore,
) -> Result<DrastcPrecomputeStats, JobsError> {
    let refreshed_at = DateTime::now();
    let cutoff_mail_time = ranking_cutoff_mail_time(refreshed_at);
    let legendary_ids = legendary_commander_ids()?;
    let aggregation = read_drastc_aggregation(
        reports_store.battle_collection(),
        &legendary_ids,
        cutoff_mail_time,
    )
    .await?;
    let supported_pairings = supported_drastc_pairings(&legendary_ids);
    let scores = build_drastc_scores_from_aggregates(
        &aggregation.observed,
        &supported_pairings,
        aggregation.reference_ranges,
    );
    let confidences = read_pairing_confidences(
        reports_store.battle_collection(),
        &supported_pairings,
        cutoff_mail_time,
    )
    .await?;
    let documents = scores
        .iter()
        .filter_map(|(key, score)| {
            confidences.get(key).map(|confidence| {
                (*key, build_drastc_document(*key, score, confidence, refreshed_at))
            })
        })
        .collect();
    let output = reports_store.precomputed_drastc_collection();
    let documents_written = write_documents(output, documents).await?;
    output.delete_many(doc! { "refreshed_at": { "$ne": refreshed_at } }).await?;
    let documents_stored = validate_materialized_data(output, documents_written).await?;

    Ok(DrastcPrecomputeStats {
        legendary_commanders: legendary_ids.len(),
        observed_pairings: aggregation.observed.len(),
        supported_pairings: supported_pairings.len(),
        scored_pairings: scores.len(),
        confidence_scored_pairings: confidences.len(),
        documents_written,
        documents_stored,
    })
}

fn ranking_cutoff_mail_time(run_at: DateTime) -> i64 {
    run_at
        .timestamp_millis()
        .saturating_sub(RANKING_WINDOW_DAYS * MILLIS_PER_DAY)
        .saturating_mul(1_000)
}

async fn write_documents(
    output: &Collection<Document>,
    documents: Vec<(PairingKey, Document)>,
) -> Result<usize, JobsError> {
    let mut written = 0;
    let mut models = Vec::with_capacity(BULK_WRITE_BATCH_SIZE);
    for (key, document) in documents {
        let mut model = output.replace_one_model(pairing_selector(key), &document)?;
        model.upsert = Some(true);
        models.push(model);
        if models.len() == BULK_WRITE_BATCH_SIZE {
            let batch_size = models.len();
            output.client().bulk_write(models).ordered(false).await?;
            written += batch_size;
            models = Vec::with_capacity(BULK_WRITE_BATCH_SIZE);
        }
    }
    if !models.is_empty() {
        let batch_size = models.len();
        output.client().bulk_write(models).ordered(false).await?;
        written += batch_size;
    }
    Ok(written)
}

async fn validate_materialized_data(
    output: &Collection<Document>,
    expected_count: usize,
) -> Result<u64, JobsError> {
    let count = validate_stored_documents(output).await?;
    let expected_count = u64::try_from(expected_count)
        .map_err(|_| JobsError::InvalidDrastcData("document count overflowed u64".into()))?;
    if count != expected_count {
        return Err(JobsError::InvalidDrastcData(format!(
            "wrote {expected_count} documents but collection contains {count}"
        )));
    }
    Ok(count)
}

async fn validate_stored_documents(output: &Collection<Document>) -> Result<u64, JobsError> {
    let count = output.count_documents(doc! {}).await?;
    if count == 0 {
        return Err(JobsError::InvalidDrastcData("collection is empty".into()));
    }
    let invalid = output
        .count_documents(doc! {
            "$or": [
                { "primary_commander_id": { "$exists": false } },
                { "secondary_commander_id": { "$exists": false } },
                { "drastc": { "$not": { "$type": "object" } } },
            ]
        })
        .await?;
    if invalid > 0 {
        return Err(JobsError::InvalidDrastcData(format!(
            "collection contains {invalid} malformed documents"
        )));
    }
    Ok(count)
}

fn pairing_selector(key: PairingKey) -> Document {
    doc! {
        "primary_commander_id": key.primary_commander_id,
        "secondary_commander_id": key.secondary_commander_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranking_cutoff_preserves_the_utc_run_time() {
        let run_at = DateTime::parse_rfc3339_str("2026-08-12T08:00:00Z").expect("run timestamp");
        let expected = DateTime::parse_rfc3339_str("2025-08-12T08:00:00Z")
            .expect("cutoff timestamp")
            .timestamp_millis()
            * 1_000;
        assert_eq!(ranking_cutoff_mail_time(run_at), expected);
    }
}
