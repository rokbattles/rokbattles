//! Precompute aggregate reward data for Karuak Ceremony bosses.

use std::collections::{BTreeMap, BTreeSet};

use core_bson::{bson_to_i64_exact, nested_i64_exact as nested_i64};
use futures::StreamExt;
use mongodb::{
    Collection,
    bson::{Bson, DateTime, Document, doc},
};
use rokbattles_api::db::ReportsStore;

use crate::error::JobsError;

const BOSS_IDS: [i64; 5] = [30_001, 30_002, 30_003, 30_004, 30_005];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KaruakCeremonyPrecomputeStats {
    pub documents_read: usize,
    pub results_counted: usize,
    pub documents_written: usize,
}

pub async fn precompute_karuak_ceremony_data(
    reports_store: &ReportsStore,
) -> Result<KaruakCeremonyPrecomputeStats, JobsError> {
    let (aggregates, mut stats) =
        read_observed_reports(reports_store.event_member_loot_report_collection()).await?;
    let output = reports_store.precomputed_karuak_ceremony_collection();
    let refreshed_at = DateTime::now();

    for boss_id in BOSS_IDS {
        let aggregate = aggregates.get(&boss_id).cloned().unwrap_or_default();
        output
            .replace_one(
                doc! { "kind": boss_id },
                build_document(boss_id, &aggregate, refreshed_at),
            )
            .upsert(true)
            .await?;
        stats.documents_written += 1;
    }

    output.delete_many(doc! { "kind": { "$nin": BOSS_IDS.to_vec() } }).await?;
    Ok(stats)
}

async fn read_observed_reports(
    source: &Collection<Document>,
) -> Result<(BTreeMap<i64, AggregateStats>, KaruakCeremonyPrecomputeStats), JobsError> {
    let mut cursor = source
        .find(doc! { "boss.id": { "$in": BOSS_IDS.to_vec() } })
        .projection(doc! { "_id": 0, "boss.id": 1, "participants.loot": 1 })
        .await?;
    let mut aggregates = BTreeMap::new();
    let mut stats = KaruakCeremonyPrecomputeStats::default();

    while let Some(next) = cursor.next().await {
        stats.documents_read += 1;
        let document = next?;
        let Some(boss_id) = nested_i64(&document, &["boss", "id"]) else { continue };
        let aggregate = aggregates.entry(boss_id).or_default();
        stats.results_counted += accumulate_document(&document, aggregate);
    }
    Ok((aggregates, stats))
}

fn accumulate_document(document: &Document, aggregate: &mut AggregateStats) -> usize {
    let Some(Bson::Array(participants)) = document.get("participants") else { return 0 };
    let mut counted = 0;
    for participant in participants.iter().filter_map(Bson::as_document) {
        let Some(Bson::Array(loot)) = participant.get("loot") else { continue };
        aggregate.results += 1;
        counted += 1;
        let mut seen = BTreeSet::new();
        for reward in loot.iter().filter_map(Bson::as_document) {
            let (Some(reward_type), Some(sub_type), Some(quantity)) = (
                direct_i32(reward, "type"),
                direct_i32(reward, "sub_type"),
                direct_i64(reward, "value"),
            ) else {
                continue;
            };
            let key = LootKey { reward_type, sub_type };
            aggregate.loot.entry(key).or_default().record(quantity, seen.insert(key));
        }
    }
    counted
}

fn build_document(kind: i64, stats: &AggregateStats, refreshed_at: DateTime) -> Document {
    let loot = stats
        .loot
        .iter()
        .map(|(key, value)| {
            doc! {
                "type": key.reward_type,
                "sub_type": key.sub_type,
                "results": usize_to_i64(value.seen),
                "drop_rate": rate(value.seen, stats.results),
                "quantity": { "min": value.min.unwrap_or(0), "max": value.max.unwrap_or(0) },
                "total_quantity": value.total_quantity,
                "average_quantity": rate_i64(value.total_quantity, value.seen),
            }
        })
        .collect::<Vec<_>>();
    doc! {
        "kind": kind,
        "loot": loot,
        "totals": { "results": usize_to_i64(stats.results) },
        "refreshed_at": refreshed_at,
    }
}

#[derive(Debug, Clone, Default)]
struct AggregateStats {
    results: usize,
    loot: BTreeMap<LootKey, LootStats>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct LootKey {
    reward_type: i32,
    sub_type: i32,
}

#[derive(Debug, Clone, Default)]
struct LootStats {
    seen: usize,
    total_quantity: i64,
    min: Option<i64>,
    max: Option<i64>,
}

impl LootStats {
    fn record(&mut self, quantity: i64, first_for_result: bool) {
        if first_for_result {
            self.seen += 1;
        }
        self.total_quantity += quantity;
        self.min = Some(self.min.map_or(quantity, |value| value.min(quantity)));
        self.max = Some(self.max.map_or(quantity, |value| value.max(quantity)));
    }
}

fn direct_i32(document: &Document, key: &str) -> Option<i32> {
    direct_i64(document, key).and_then(|v| i32::try_from(v).ok())
}
fn direct_i64(document: &Document, key: &str) -> Option<i64> {
    document.get(key).and_then(bson_to_i64_exact)
}
fn rate(part: usize, total: usize) -> f64 {
    if total == 0 { 0.0 } else { part as f64 / total as f64 }
}
fn rate_i64(total: i64, count: usize) -> f64 {
    if count == 0 { 0.0 } else { total as f64 / count as f64 }
}
fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{DateTime, doc};

    use super::*;

    #[test]
    fn aggregates_each_participant_as_one_result() {
        let mut aggregate = AggregateStats::default();
        let counted = accumulate_document(
            &doc! { "participants": [
                { "loot": [{ "type": 2, "sub_type": 17, "value": 1 }] },
                { "loot": [{ "type": 2, "sub_type": 17, "value": 2 }, { "type": 2, "sub_type": 149, "value": 10 }] },
            ] },
            &mut aggregate,
        );
        assert_eq!(counted, 2);
        assert_eq!(aggregate.results, 2);
        assert_eq!(aggregate.loot[&LootKey { reward_type: 2, sub_type: 17 }].seen, 2);
    }

    #[test]
    fn builds_poolless_boss_document_with_totals() {
        let mut aggregate = AggregateStats { results: 2, ..Default::default() };
        aggregate.loot.entry(LootKey { reward_type: 2, sub_type: 17 }).or_default().record(1, true);
        let document = build_document(30_001, &aggregate, DateTime::from_millis(123));
        assert_eq!(document.get_i64("kind"), Ok(30_001));
        assert_eq!(document.get_document("totals").unwrap().get_i64("results"), Ok(2));
        assert!(document.get_array("loot").is_ok());
        assert!(!document.contains_key("pools"));
    }
}
