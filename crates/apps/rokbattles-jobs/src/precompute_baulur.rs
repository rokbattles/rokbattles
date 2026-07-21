//! Precompute aggregate reward data for Baulur.

use std::collections::BTreeMap;

use futures::StreamExt;
use mongodb::{
    Collection,
    bson::{Bson, DateTime, Document, doc},
};
use rokbattles_api::db::ReportsStore;
use rokbattles_bson::{
    bson_to_f64, bson_to_i32_exact as bson_to_i32, bson_to_i64_exact as bson_to_i64,
};

use crate::error::JobsError;

const BAULUR_TARGET_KINDS: [i32; 2] = [102_000_055, 102_000_063];

/// Counts from one Baulur precompute run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BaulurPrecomputeStats {
    pub documents_read: usize,
    pub results_counted: usize,
    pub documents_written: usize,
}

/// Refresh Baulur precomputed reward pool documents.
pub async fn precompute_baulur_data(
    reports_store: &ReportsStore,
) -> Result<BaulurPrecomputeStats, JobsError> {
    let (mut aggregate, mut stats) =
        read_observed_baulur_reports(reports_store.barcanyonkillboss_collection()).await?;
    let precomputed = reports_store.precomputed_baulur_collection();
    let refreshed_at = DateTime::now();

    for target in target_catalog() {
        let kind_stats = aggregate.remove(&target.kind).unwrap_or_default();
        let document = build_precomputed_document(target, &kind_stats, refreshed_at);
        let selector = target.selector();
        precomputed.replace_one(selector, document).upsert(true).await?;
        stats.documents_written += 1;
    }

    Ok(stats)
}

async fn read_observed_baulur_reports(
    source: &Collection<Document>,
) -> Result<(BTreeMap<i32, KindStats>, BaulurPrecomputeStats), JobsError> {
    let mut cursor = source
        .find(doc! {
            "npc.type": { "$in": BAULUR_TARGET_KINDS.to_vec() },
        })
        .projection(doc! {
            "_id": 0,
            "npc.type": 1,
            "participants.damage_rate": 1,
            "participants.loot": 1,
        })
        .await?;

    let mut aggregate = target_catalog()
        .into_iter()
        .map(|target| (target.kind, KindStats::default()))
        .collect::<BTreeMap<_, _>>();
    let mut stats = BaulurPrecomputeStats::default();

    while let Some(next) = cursor.next().await {
        stats.documents_read += 1;
        let document = next?;
        stats.results_counted += accumulate_report_document(&document, &mut aggregate);
    }

    Ok((aggregate, stats))
}

fn accumulate_report_document(
    document: &Document,
    aggregate: &mut BTreeMap<i32, KindStats>,
) -> usize {
    let Some(kind) = nested_i32(document, &["npc", "type"]) else {
        return 0;
    };
    let Some(kind_stats) = aggregate.get_mut(&kind) else {
        return 0;
    };
    let Some(Bson::Array(participants)) = nested_bson(document, &["participants"]) else {
        return 0;
    };

    let mut counted = 0;
    for participant in participants {
        let Some(participant_document) = participant.as_document() else {
            continue;
        };
        let Some(damage_rate) = direct_f64(participant_document, "damage_rate") else {
            continue;
        };

        let pool_kind = DamagePoolKind::from_damage_rate(damage_rate);
        let pool_stats = kind_stats.pools.entry(pool_kind).or_default();
        kind_stats.results += 1;
        pool_stats.results += 1;
        pool_stats.damage_rate.record(damage_rate);

        if let Some(Bson::Array(loot)) = participant_document.get("loot") {
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

                pool_stats
                    .loot
                    .entry(LootKey { reward_type, sub_type })
                    .or_default()
                    .record(quantity);
            }
        }

        counted += 1;
    }

    counted
}

fn build_precomputed_document(
    target: TargetMetadata,
    stats: &KindStats,
    refreshed_at: DateTime,
) -> Document {
    let results = usize_to_i64(stats.results);
    let loot_pools = DamagePoolKind::all()
        .iter()
        .map(|pool| {
            let pool_stats = stats.pools.get(pool).cloned().unwrap_or_default();
            build_pool_document(*pool, &pool_stats, stats.results)
        })
        .collect::<Vec<_>>();

    doc! {
        "kind": target.kind,
        "loot_pools": loot_pools,
        "totals": {
            "results": results,
        },
        "refreshed_at": refreshed_at,
    }
}

fn build_pool_document(pool: DamagePoolKind, stats: &PoolStats, kind_results: usize) -> Document {
    let loot = stats
        .loot
        .iter()
        .map(|(key, loot_stats)| build_loot_document(*key, loot_stats, stats.results))
        .collect::<Vec<_>>();

    doc! {
        "pool": pool.value(),
        "results": usize_to_i64(stats.results),
        "receive_rate": rate(stats.results, kind_results),
        "damage_factor": stats.damage_rate.to_document(),
        "loot": loot,
    }
}

fn build_loot_document(key: LootKey, stats: &LootStats, pool_results: usize) -> Document {
    doc! {
        "type": key.reward_type,
        "sub_type": key.sub_type,
        "results": usize_to_i64(stats.seen),
        "drop_rate": rate(stats.seen, pool_results),
        "quantity": {
            "min": stats.quantity.min.unwrap_or(0),
            "max": stats.quantity.max.unwrap_or(0),
        },
        "total_quantity": stats.total_quantity,
        "average_quantity": rate_i64(stats.total_quantity, stats.seen),
    }
}

fn target_catalog() -> Vec<TargetMetadata> {
    BAULUR_TARGET_KINDS.into_iter().map(|kind| TargetMetadata { kind }).collect()
}

#[derive(Debug, Clone, Copy)]
struct TargetMetadata {
    kind: i32,
}

impl TargetMetadata {
    fn selector(self) -> Document {
        doc! { "kind": self.kind }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DamagePoolKind {
    UnderOnePercent,
    OneToOneHundredPercent,
}

impl DamagePoolKind {
    const fn all() -> &'static [Self; 2] {
        &[Self::UnderOnePercent, Self::OneToOneHundredPercent]
    }

    fn from_damage_rate(damage_rate: f64) -> Self {
        if damage_rate < 1.0 { Self::UnderOnePercent } else { Self::OneToOneHundredPercent }
    }

    const fn value(self) -> i32 {
        match self {
            Self::UnderOnePercent => 0,
            Self::OneToOneHundredPercent => 1,
        }
    }
}

#[derive(Debug, Default)]
struct KindStats {
    results: usize,
    pools: BTreeMap<DamagePoolKind, PoolStats>,
}

#[derive(Debug, Clone, Default)]
struct PoolStats {
    results: usize,
    damage_rate: NumericRange,
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
    quantity: IntegerRange,
}

impl LootStats {
    fn record(&mut self, quantity: i64) {
        self.seen += 1;
        self.total_quantity += quantity;
        self.quantity.record(quantity);
    }
}

#[derive(Debug, Clone, Default)]
struct NumericRange {
    min: Option<f64>,
    max: Option<f64>,
}

impl NumericRange {
    fn record(&mut self, value: f64) {
        self.min = Some(self.min.map_or(value, |current| current.min(value)));
        self.max = Some(self.max.map_or(value, |current| current.max(value)));
    }

    fn to_document(&self) -> Document {
        doc! {
            "min": optional_f64(self.min),
            "max": optional_f64(self.max),
        }
    }
}

#[derive(Debug, Clone, Default)]
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

fn nested_bson<'a>(document: &'a Document, path: &[&str]) -> Option<&'a Bson> {
    let (last, parents) = path.split_last()?;
    let mut current = document;

    for key in parents {
        current = current.get_document(key).ok()?;
    }

    current.get(*last)
}

fn nested_i32(document: &Document, path: &[&str]) -> Option<i32> {
    nested_bson(document, path).and_then(bson_to_i32)
}

fn direct_i32(document: &Document, key: &str) -> Option<i32> {
    document.get(key).and_then(bson_to_i32)
}

fn direct_i64(document: &Document, key: &str) -> Option<i64> {
    document.get(key).and_then(bson_to_i64)
}

fn direct_f64(document: &Document, key: &str) -> Option<f64> {
    document.get(key).and_then(bson_to_f64)
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

fn optional_f64(value: Option<f64>) -> Bson {
    value.map(Bson::Double).unwrap_or(Bson::Null)
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{Bson, doc};

    use super::*;

    #[test]
    fn target_catalog_contains_only_requested_baulurs() {
        let targets = target_catalog();

        assert_eq!(targets.len(), 2);
        assert!(targets.iter().any(|target| target.kind == 102_000_055));
        assert!(targets.iter().any(|target| target.kind == 102_000_063));
    }

    #[test]
    fn damage_pool_kind_splits_below_one_percent() {
        assert_eq!(DamagePoolKind::from_damage_rate(0.99), DamagePoolKind::UnderOnePercent);
        assert_eq!(DamagePoolKind::from_damage_rate(1.0), DamagePoolKind::OneToOneHundredPercent);
    }

    #[test]
    fn accumulate_report_document_groups_participant_loot_by_kind_and_damage_pool() {
        let mut aggregate = target_catalog()
            .into_iter()
            .map(|target| (target.kind, KindStats::default()))
            .collect::<BTreeMap<_, _>>();
        let report = doc! {
            "npc": { "type": 102_000_055 },
            "participants": [
                {
                    "damage_rate": 0.75,
                    "loot": [
                        { "type": 2, "sub_type": 26, "value": 1 },
                    ],
                },
                {
                    "damage_rate": 12.5,
                    "loot": [
                        { "type": 2, "sub_type": 65, "value": 2 },
                    ],
                },
            ],
        };

        assert_eq!(accumulate_report_document(&report, &mut aggregate), 2);

        let stats = aggregate.get(&102_000_055).expect("kind stats");
        assert_eq!(stats.results, 2);
        let under_one = stats.pools.get(&DamagePoolKind::UnderOnePercent).expect("under one");
        assert_eq!(under_one.results, 1);
        assert_eq!(under_one.damage_rate.min, Some(0.75));
        let regular =
            stats.pools.get(&DamagePoolKind::OneToOneHundredPercent).expect("one to one hundred");
        assert_eq!(regular.results, 1);
        assert_eq!(regular.loot.len(), 1);
    }

    #[test]
    fn build_precomputed_document_has_kind_pools_and_result_total_only() {
        let target = TargetMetadata { kind: 102_000_055 };
        let mut stats = KindStats { results: 2, ..KindStats::default() };
        let pool = stats.pools.entry(DamagePoolKind::OneToOneHundredPercent).or_default();
        pool.results = 2;
        pool.damage_rate.record(8.5);
        pool.damage_rate.record(12.0);
        pool.loot.entry(LootKey { reward_type: 2, sub_type: 65 }).or_default().record(2);

        let document = build_precomputed_document(target, &stats, DateTime::now());

        assert_eq!(document.get_i32("kind"), Ok(102_000_055));
        assert!(!document.contains_key("level"));
        assert!(!document.contains_key("data"));
        assert_eq!(
            document.get_document("totals").and_then(|totals| totals.get_i64("results")),
            Ok(2)
        );
        let totals = document.get_document("totals").expect("totals");
        assert!(!totals.contains_key("ap_used"));
        assert!(!totals.contains_key("honor_points_gained"));
        assert!(!totals.contains_key("xp_gained"));
        let pools = document.get_array("loot_pools").expect("loot pools");
        assert_eq!(pools.len(), 2);
        let pool = pools.get(1).and_then(Bson::as_document).expect("pool");
        assert_eq!(pool.get_i32("pool"), Ok(1));
        assert_eq!(pool.get_i64("results"), Ok(2));
        assert_eq!(pool.get_f64("receive_rate"), Ok(1.0));
        let loot = pool.get_array("loot").expect("loot");
        let item = loot.first().and_then(Bson::as_document).expect("item");
        assert_eq!(item.get_i64("results"), Ok(1));
        assert_eq!(item.get_f64("drop_rate"), Ok(0.5));
    }
}
