//! Precompute aggregate reward data for Kahar treasure.

use std::collections::BTreeMap;

use futures::StreamExt;
use mongodb::{
    Collection,
    bson::{Bson, DateTime, Document, doc},
};
use rokbattles_api::db::ReportsStore;

use crate::error::JobsError;

const KAHAR_TREASURE_AGGREGATE_KEY: &str = "all";

/// Counts from one Kahar treasure precompute run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KaharTreasurePrecomputeStats {
    pub documents_read: usize,
    pub mails_counted: usize,
    pub documents_written: usize,
}

/// Refresh Kahar treasure precomputed reward documents.
pub async fn precompute_kahar_treasure_data(
    reports_store: &ReportsStore,
) -> Result<KaharTreasurePrecomputeStats, JobsError> {
    let (aggregate, mut stats) =
        read_observed_kahar_treasure_reports(reports_store.system_kahar_treasure_collection())
            .await?;
    let precomputed = reports_store.precomputed_kahar_treasure_collection();
    let refreshed_at = DateTime::now();
    let document = build_precomputed_document(&aggregate, refreshed_at);

    precomputed
        .replace_one(doc! { "key": KAHAR_TREASURE_AGGREGATE_KEY }, document)
        .upsert(true)
        .await?;
    stats.documents_written += 1;

    Ok(stats)
}

async fn read_observed_kahar_treasure_reports(
    source: &Collection<Document>,
) -> Result<(AggregateStats, KaharTreasurePrecomputeStats), JobsError> {
    let mut cursor = source
        .find(doc! {})
        .projection(doc! {
            "_id": 0,
            "loot": 1,
        })
        .await?;

    let mut aggregate = AggregateStats::default();
    let mut stats = KaharTreasurePrecomputeStats::default();

    while let Some(next) = cursor.next().await {
        stats.documents_read += 1;
        let document = next?;
        accumulate_report_document(&document, &mut aggregate);
        stats.mails_counted += 1;
    }

    Ok((aggregate, stats))
}

fn accumulate_report_document(document: &Document, aggregate: &mut AggregateStats) {
    aggregate.results += 1;

    let Some(Bson::Array(loot)) = document.get("loot") else {
        return;
    };

    for reward in loot {
        let Some(reward_document) = reward.as_document() else {
            continue;
        };
        let Some(reward_type) = direct_i32(reward_document, "type") else {
            continue;
        };
        let Some(sub_type) = direct_i32(reward_document, "sub_type") else {
            continue;
        };
        let Some(quantity) = direct_i64(reward_document, "value") else {
            continue;
        };

        aggregate.loot.entry(LootKey { reward_type, sub_type }).or_default().record(quantity);
    }
}

fn build_precomputed_document(stats: &AggregateStats, refreshed_at: DateTime) -> Document {
    let loot = stats
        .loot
        .iter()
        .map(|(key, loot_stats)| build_loot_document(*key, loot_stats, stats.results))
        .collect::<Vec<_>>();

    doc! {
        "key": KAHAR_TREASURE_AGGREGATE_KEY,
        "loot": loot,
        "totals": {
            "results": usize_to_i64(stats.results),
        },
        "refreshed_at": refreshed_at,
    }
}

fn build_loot_document(key: LootKey, stats: &LootStats, results: usize) -> Document {
    doc! {
        "type": key.reward_type,
        "sub_type": key.sub_type,
        "results": usize_to_i64(stats.seen),
        "drop_rate": rate(stats.seen, results),
        "quantity": {
            "min": stats.quantity.min.unwrap_or(0),
            "max": stats.quantity.max.unwrap_or(0),
        },
        "total_quantity": stats.total_quantity,
        "average_quantity": rate_i64(stats.total_quantity, stats.seen),
    }
}

#[derive(Debug, Default)]
struct AggregateStats {
    results: usize,
    loot: BTreeMap<LootKey, LootStats>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct LootKey {
    reward_type: i32,
    sub_type: i32,
}

#[derive(Debug, Default)]
struct LootStats {
    seen: usize,
    total_quantity: i64,
    quantity: IntegerRange,
}

impl LootStats {
    fn record(&mut self, quantity: i64) {
        self.seen += 1;
        self.total_quantity += quantity;
        self.quantity.record(quantity);
    }
}

#[derive(Debug, Default)]
struct IntegerRange {
    min: Option<i64>,
    max: Option<i64>,
}

impl IntegerRange {
    fn record(&mut self, value: i64) {
        self.min = Some(self.min.map_or(value, |current| current.min(value)));
        self.max = Some(self.max.map_or(value, |current| current.max(value)));
    }
}

fn direct_i32(document: &Document, key: &str) -> Option<i32> {
    document.get(key).and_then(bson_to_i32)
}

fn direct_i64(document: &Document, key: &str) -> Option<i64> {
    document.get(key).and_then(bson_to_i64)
}

fn bson_to_i32(value: &Bson) -> Option<i32> {
    match value {
        Bson::Int32(value) => Some(*value),
        Bson::Int64(value) => i32::try_from(*value).ok(),
        Bson::Double(value) if value.fract() == 0.0 => i32::try_from(*value as i64).ok(),
        _ => None,
    }
}

fn bson_to_i64(value: &Bson) -> Option<i64> {
    match value {
        Bson::Int32(value) => Some(i64::from(*value)),
        Bson::Int64(value) => Some(*value),
        Bson::Double(value) if value.fract() == 0.0 => Some(*value as i64),
        _ => None,
    }
}

fn rate(part: usize, total: usize) -> f64 {
    if total == 0 { 0.0 } else { part as f64 / total as f64 }
}

fn rate_i64(total_quantity: i64, seen: usize) -> f64 {
    if seen == 0 { 0.0 } else { total_quantity as f64 / seen as f64 }
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{Bson, doc};

    use super::*;

    #[test]
    fn accumulate_report_document_counts_each_mail_as_one_result() {
        let mut aggregate = AggregateStats::default();
        let report = doc! {
            "loot": [
                { "type": 1, "sub_type": 9, "value": 45_000 },
                { "type": 2, "sub_type": 147, "value": 5 },
            ],
        };

        accumulate_report_document(&report, &mut aggregate);

        assert_eq!(aggregate.results, 1);
        assert_eq!(aggregate.loot.len(), 2);
    }

    #[test]
    fn accumulate_report_document_groups_loot_across_all_mails() {
        let mut aggregate = AggregateStats::default();

        accumulate_report_document(
            &doc! {
                "loot": [
                    { "type": 2, "sub_type": 147, "value": 5 },
                    { "type": 2, "sub_type": 10, "value": 1 },
                ],
            },
            &mut aggregate,
        );
        accumulate_report_document(
            &doc! {
                "loot": [
                    { "type": 2, "sub_type": 147, "value": 8 },
                ],
            },
            &mut aggregate,
        );

        let stats =
            aggregate.loot.get(&LootKey { reward_type: 2, sub_type: 147 }).expect("loot stats");

        assert_eq!(stats.seen, 2);
        assert_eq!(stats.total_quantity, 13);
        assert_eq!(stats.quantity.min, Some(5));
        assert_eq!(stats.quantity.max, Some(8));
    }

    #[test]
    fn build_precomputed_document_has_one_poolless_total_result_shape() {
        let mut stats = AggregateStats { results: 2, ..AggregateStats::default() };
        stats.loot.entry(LootKey { reward_type: 2, sub_type: 147 }).or_default().record(5);

        let document = build_precomputed_document(&stats, DateTime::now());

        assert_eq!(document.get_str("key"), Ok(KAHAR_TREASURE_AGGREGATE_KEY));
        assert!(!document.contains_key("data"));
        assert!(!document.contains_key("loot_pools"));
        assert_eq!(
            document.get_document("totals").and_then(|totals| totals.get_i64("results")),
            Ok(2)
        );
        let totals = document.get_document("totals").expect("totals");
        assert!(!totals.contains_key("ap_used"));
        assert!(!totals.contains_key("honor_points_gained"));
        assert!(!totals.contains_key("xp_gained"));
        let loot = document.get_array("loot").expect("loot");
        let item = loot.first().and_then(Bson::as_document).expect("item");
        assert_eq!(item.get_i64("results"), Ok(1));
        assert_eq!(item.get_f64("drop_rate"), Ok(0.5));
        assert_eq!(item.get_i64("total_quantity"), Ok(5));
        assert_eq!(item.get_f64("average_quantity"), Ok(5.0));
    }
}
