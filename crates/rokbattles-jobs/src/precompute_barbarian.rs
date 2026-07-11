//! Precompute aggregate reward data for barbarians.

use std::collections::{BTreeMap, BTreeSet};

use core_bson::{bson_to_i32_exact as bson_to_i32, bson_to_i64_exact as bson_to_i64};
use futures::StreamExt;
use mongodb::{
    Collection,
    bson::{Bson, DateTime, Document, doc},
};
use rokbattles_api::db::ReportsStore;

use crate::error::JobsError;

const BARBARIAN_B_TYPE: i32 = 1;
const MARAUDER_B_TYPE: i32 = 15;

/// Counts from one barbarian/marauder precompute run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BarbarianPrecomputeStats {
    pub documents_read: usize,
    pub reports_counted: usize,
    pub documents_written: usize,
}

/// Refresh barbarian and marauder precomputed reward documents.
pub async fn precompute_barbarian_data(
    reports_store: &ReportsStore,
) -> Result<BarbarianPrecomputeStats, JobsError> {
    let targets = target_catalog();
    let target_ids = targets.iter().map(|target| target.kind).collect::<BTreeSet<_>>();
    let target_by_kind =
        targets.iter().map(|target| (target.kind, *target)).collect::<BTreeMap<_, _>>();
    let (mut aggregate, mut stats) =
        read_observed_barbarian_reports(reports_store.battle_collection(), &target_by_kind).await?;
    let precomputed = reports_store.precomputed_barbarian_collection();
    let refreshed_at = DateTime::now();

    for target in targets {
        let key = target.key();
        let level_stats = aggregate.remove(&key).unwrap_or_default();

        let document = build_precomputed_document(target, &level_stats, refreshed_at);
        let selector = target.selector();
        precomputed.replace_one(selector, document).upsert(true).await?;
        stats.documents_written += 1;
    }

    debug_assert_eq!(target_ids.len(), stats.documents_written);

    Ok(stats)
}

async fn read_observed_barbarian_reports(
    source: &Collection<Document>,
    target_by_kind: &BTreeMap<i32, TargetMetadata>,
) -> Result<(BTreeMap<TargetKey, LevelStats>, BarbarianPrecomputeStats), JobsError> {
    let mut cursor = source
        .find(doc! {
            "opponents": {
                "$elemMatch": {
                    "player_id": -2,
                    "npc.type": { "$in": target_by_kind.keys().copied().collect::<Vec<_>>() },
                    "npc.b_type": { "$in": [BARBARIAN_B_TYPE, MARAUDER_B_TYPE] },
                },
            },
        })
        .projection(doc! {
            "_id": 0,
            "opponents.player_id": 1,
            "opponents.npc.type": 1,
            "opponents.npc.b_type": 1,
            "opponents.npc.experience": 1,
            "opponents.npc.loot": 1,
        })
        .await?;

    let mut aggregate = target_by_kind
        .values()
        .map(|target| (target.key(), LevelStats::default()))
        .collect::<BTreeMap<_, _>>();
    let mut stats = BarbarianPrecomputeStats::default();

    while let Some(next) = cursor.next().await {
        stats.documents_read += 1;
        let document = next?;
        stats.reports_counted +=
            accumulate_report_document(&document, target_by_kind, &mut aggregate);
    }

    Ok((aggregate, stats))
}

fn accumulate_report_document(
    document: &Document,
    target_by_kind: &BTreeMap<i32, TargetMetadata>,
    aggregate: &mut BTreeMap<TargetKey, LevelStats>,
) -> usize {
    let Some(Bson::Array(opponents)) = nested_bson(document, &["opponents"]) else {
        return 0;
    };

    let mut counted = 0;
    for opponent in opponents {
        let Some(opponent_document) = opponent.as_document() else {
            continue;
        };
        if direct_i64(opponent_document, "player_id") != Some(-2) {
            continue;
        }

        let Some(npc) = opponent_document.get_document("npc").ok() else {
            continue;
        };
        let Some(kind) = direct_i32(npc, "type") else {
            continue;
        };
        let Some(target) = target_by_kind.get(&kind).copied() else {
            continue;
        };
        if direct_i32(npc, "b_type") != Some(target.b_type) {
            continue;
        }

        let key = target.key();
        let Some(level_stats) = aggregate.get_mut(&key) else {
            continue;
        };

        level_stats.results += 1;
        if let Some(experience) = direct_i64(npc, "experience") {
            level_stats.xp_gained += experience;
        }

        if let Some(Bson::Array(loot)) = npc.get("loot") {
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

                level_stats
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
    stats: &LevelStats,
    refreshed_at: DateTime,
) -> Document {
    let results = usize_to_i64(stats.results);
    let loot = stats
        .loot
        .iter()
        .map(|(key, loot_stats)| build_loot_document(*key, loot_stats, stats.results))
        .collect::<Vec<_>>();

    doc! {
        "kind": target.kind,
        "level": target.level,
        "loot": loot,
        "data": {
            "b_type": target.b_type,
            "ap_cost": target.ap_cost,
            "honor_points": target.honor_points,
            "base_xp": target.base_xp,
        },
        "totals": {
            "results": results,
            "ap_used": results * i64::from(target.ap_cost),
            "honor_points_gained": results * i64::from(target.honor_points),
            "xp_gained": stats.xp_gained,
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

fn target_catalog() -> Vec<TargetMetadata> {
    let mut targets = Vec::with_capacity(192);

    push_level_range(&mut targets, 1, 40, 1);
    push_level_range(&mut targets, 401, 415, 41);
    push_level_range(&mut targets, 701, 740, 1);
    push_level_range(&mut targets, 801, 840, 1);
    push_level_range(&mut targets, 901, 940, 1);
    push_level_range(&mut targets, 150_009, 150_023, 41);
    targets.push(TargetMetadata {
        kind: 99,
        b_type: MARAUDER_B_TYPE,
        level: 1,
        ap_cost: 50,
        honor_points: 0,
        base_xp: 3000,
    });
    targets.push(TargetMetadata {
        kind: 100,
        b_type: MARAUDER_B_TYPE,
        level: 41,
        ap_cost: 80,
        honor_points: honor_points_for_level(41),
        base_xp: 8200,
    });

    targets
}

fn push_level_range(
    targets: &mut Vec<TargetMetadata>,
    first_id: i32,
    last_id: i32,
    first_level: i32,
) {
    for kind in first_id..=last_id {
        let level = first_level + kind - first_id;
        targets.push(target_for_level(kind, BARBARIAN_B_TYPE, level));
    }
}

fn target_for_level(kind: i32, b_type: i32, level: i32) -> TargetMetadata {
    TargetMetadata {
        kind,
        b_type,
        level,
        ap_cost: ap_cost_for_level(level),
        honor_points: honor_points_for_level(level),
        base_xp: base_xp_for_level(level),
    }
}

fn ap_cost_for_level(level: i32) -> i32 {
    if level >= 41 { 80 } else { 50 }
}

fn honor_points_for_level(level: i32) -> i32 {
    match level {
        41..=45 => 10,
        46..=50 => 16,
        51..=55 => 20,
        _ => 0,
    }
}

fn base_xp_for_level(level: i32) -> i32 {
    if level >= 41 { level * 200 } else { level * 100 }
}

#[derive(Debug, Clone, Copy)]
struct TargetMetadata {
    kind: i32,
    b_type: i32,
    level: i32,
    ap_cost: i32,
    honor_points: i32,
    base_xp: i32,
}

impl TargetMetadata {
    fn key(self) -> TargetKey {
        TargetKey { kind: self.kind, level: self.level }
    }

    fn selector(self) -> Document {
        doc! {
            "kind": self.kind,
            "level": self.level,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TargetKey {
    kind: i32,
    level: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct LootKey {
    reward_type: i32,
    sub_type: i32,
}

#[derive(Debug, Default)]
struct LevelStats {
    results: usize,
    xp_gained: i64,
    loot: BTreeMap<LootKey, LootStats>,
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

fn nested_bson<'a>(document: &'a Document, path: &[&str]) -> Option<&'a Bson> {
    let (last, parents) = path.split_last()?;
    let mut current = document;

    for key in parents {
        current = current.get_document(key).ok()?;
    }

    current.get(*last)
}

fn direct_i32(document: &Document, key: &str) -> Option<i32> {
    document.get(key).and_then(bson_to_i32)
}

fn direct_i64(document: &Document, key: &str) -> Option<i64> {
    document.get(key).and_then(bson_to_i64)
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
    fn target_catalog_contains_expected_supported_ranges() {
        let targets = target_catalog();

        assert_eq!(targets.len(), 192);
        assert!(targets.iter().any(|target| target.kind == 40 && target.level == 40));
        assert!(targets.iter().any(|target| {
            target.kind == 99 && target.b_type == MARAUDER_B_TYPE && target.level == 1
        }));
        assert!(targets.iter().any(|target| {
            target.kind == 100 && target.b_type == MARAUDER_B_TYPE && target.level == 41
        }));
        assert!(targets.iter().any(|target| target.kind == 401 && target.level == 41));
        assert!(targets.iter().any(|target| target.kind == 415 && target.level == 55));
        assert!(targets.iter().any(|target| target.kind == 701 && target.level == 1));
        assert!(targets.iter().any(|target| target.kind == 940 && target.level == 40));
        assert!(targets.iter().any(|target| target.kind == 150_009 && target.level == 41));
        assert!(targets.iter().any(|target| target.kind == 150_023 && target.level == 55));
        assert!(!targets.iter().any(|target| target.kind == 400));
        assert!(!targets.iter().any(|target| target.kind == 416));
    }

    #[test]
    fn target_catalog_maps_ap_honor_and_base_xp_from_level() {
        let targets = target_catalog();
        let level_38 = targets.iter().find(|target| target.kind == 38).expect("level 38");
        let level_45 = targets.iter().find(|target| target.kind == 405).expect("level 45");
        let english_soldier =
            targets.iter().find(|target| target.kind == 150_009).expect("english soldier");
        let marauder = targets.iter().find(|target| target.kind == 99).expect("marauder");
        let level_41_marauder =
            targets.iter().find(|target| target.kind == 100).expect("level 41 marauder");

        assert_eq!(level_38.ap_cost, 50);
        assert_eq!(level_38.honor_points, 0);
        assert_eq!(level_38.base_xp, 3800);
        assert_eq!(level_45.ap_cost, 80);
        assert_eq!(level_45.honor_points, 10);
        assert_eq!(level_45.base_xp, 9000);
        assert_eq!(english_soldier.ap_cost, 80);
        assert_eq!(english_soldier.honor_points, 10);
        assert_eq!(english_soldier.base_xp, 8200);
        assert_eq!(marauder.ap_cost, 50);
        assert_eq!(marauder.honor_points, 0);
        assert_eq!(marauder.base_xp, 3000);
        assert_eq!(level_41_marauder.ap_cost, 80);
        assert_eq!(level_41_marauder.honor_points, 10);
        assert_eq!(level_41_marauder.base_xp, 8200);
    }

    #[test]
    fn honor_points_match_kvktask_rule_visible_values() {
        assert_eq!(honor_points_for_level(41), 10);
        assert_eq!(honor_points_for_level(45), 10);
        assert_eq!(honor_points_for_level(46), 16);
        assert_eq!(honor_points_for_level(50), 16);
        assert_eq!(honor_points_for_level(51), 20);
        assert_eq!(honor_points_for_level(55), 20);
    }

    #[test]
    fn target_catalog_maps_named_variants_to_their_base_honor_values() {
        let targets = target_catalog();

        let barbarian = targets.iter().find(|target| target.kind == 401).expect("barbarian");
        let marauder = targets.iter().find(|target| target.kind == 100).expect("marauder");
        let english_soldier =
            targets.iter().find(|target| target.kind == 150_009).expect("english soldier");

        assert_eq!(barbarian.honor_points, 10);
        assert_eq!(marauder.honor_points, barbarian.honor_points);
        assert_eq!(english_soldier.honor_points, barbarian.honor_points);
    }

    #[test]
    fn accumulate_report_document_groups_matching_barbarian_loot_by_kind() {
        let targets = target_catalog();
        let target_by_kind =
            targets.iter().map(|target| (target.kind, *target)).collect::<BTreeMap<_, _>>();
        let mut aggregate = target_by_kind
            .values()
            .map(|target| (target.key(), LevelStats::default()))
            .collect::<BTreeMap<_, _>>();
        let report = doc! {
            "opponents": [
                {
                    "player_id": -2,
                    "npc": {
                        "type": 38,
                        "b_type": 1,
                        "experience": 6650,
                        "loot": [
                            { "type": 2, "sub_type": 7005, "value": 66 },
                            { "type": 2, "sub_type": 128, "value": 38 },
                        ],
                    },
                },
                {
                    "player_id": -2,
                    "npc": {
                        "type": 99,
                        "b_type": 15,
                        "experience": 3000,
                    },
                },
            ],
        };

        assert_eq!(accumulate_report_document(&report, &target_by_kind, &mut aggregate), 2);

        let stats = aggregate.get(&TargetKey { kind: 38, level: 38 }).expect("level stats");
        assert_eq!(stats.results, 1);
        assert_eq!(stats.xp_gained, 6650);
        assert_eq!(stats.loot.len(), 2);
        let marauder_stats =
            aggregate.get(&TargetKey { kind: 99, level: 1 }).expect("marauder stats");
        assert_eq!(marauder_stats.results, 1);
        assert_eq!(marauder_stats.xp_gained, 3000);
    }

    #[test]
    fn build_precomputed_document_calculates_rates_and_totals() {
        let target = TargetMetadata {
            kind: 38,
            b_type: 1,
            level: 38,
            ap_cost: 50,
            honor_points: 0,
            base_xp: 3800,
        };
        let mut stats = LevelStats { results: 2, xp_gained: 13_300, ..LevelStats::default() };
        stats.loot.entry(LootKey { reward_type: 2, sub_type: 7005 }).or_default().record(66);

        let document = build_precomputed_document(target, &stats, DateTime::now());

        assert_eq!(document.get_i32("kind"), Ok(38));
        assert_eq!(document.get_i32("level"), Ok(38));
        assert_eq!(document.get_document("data").and_then(|data| data.get_i32("b_type")), Ok(1));
        assert_eq!(
            document.get_document("data").and_then(|data| data.get_i32("base_xp")),
            Ok(3800)
        );
        assert_eq!(
            document.get_document("totals").and_then(|totals| totals.get_i64("results")),
            Ok(2)
        );
        assert_eq!(
            document.get_document("totals").and_then(|totals| totals.get_i64("xp_gained")),
            Ok(13_300)
        );
        let loot = document.get_array("loot").expect("loot");
        let item = loot.first().and_then(Bson::as_document).expect("item");
        assert_eq!(item.get_i64("results"), Ok(1));
        assert_eq!(item.get_f64("drop_rate"), Ok(0.5));
    }
}
