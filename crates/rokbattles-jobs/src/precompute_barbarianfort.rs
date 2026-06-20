//! Precompute aggregate reward data for barbarian forts.

use std::collections::BTreeMap;

use futures::StreamExt;
use mongodb::{
    Collection,
    bson::{Bson, DateTime, Document, doc},
};
use rokbattles_api::db::ReportsStore;

use crate::error::JobsError;

const SYSTEM_BARBARIAN_FORT_SUB_TYPE: i32 = 11;
const BARBARIAN_FORT_SUB_PARAM: i32 = 1;
const MARAUDER_ENCAMPMENT_SUB_PARAM: i32 = 3;
const MOTTE_SUB_PARAM: i32 = 4;

/// Counts from one barbarian fort precompute run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BarbarianFortPrecomputeStats {
    pub documents_read: usize,
    pub reports_counted: usize,
    pub documents_written: usize,
}

/// Refresh barbarian fort precomputed reward/tier documents.
pub async fn precompute_barbarian_fort_data(
    reports_store: &ReportsStore,
) -> Result<BarbarianFortPrecomputeStats, JobsError> {
    let (mut aggregate, mut stats) =
        read_observed_fort_reports(reports_store.system_barbarian_fort_collection()).await?;
    let precomputed = reports_store.precomputed_barbarian_fort_collection();
    let refreshed_at = DateTime::now();

    for target in target_catalog() {
        let key = target.key();
        let level_stats = aggregate.remove(&key).unwrap_or_default();

        let document = build_precomputed_document(target, &level_stats, refreshed_at);
        let selector = target.selector();
        precomputed.replace_one(selector, document).upsert(true).await?;
        stats.documents_written += 1;
    }

    Ok(stats)
}

async fn read_observed_fort_reports(
    source: &Collection<Document>,
) -> Result<(BTreeMap<TargetKey, LevelStats>, BarbarianFortPrecomputeStats), JobsError> {
    let mut cursor = source
        .find(doc! {
            "body.sub_type": SYSTEM_BARBARIAN_FORT_SUB_TYPE,
            "body.sub_param": {
                "$in": [
                    BARBARIAN_FORT_SUB_PARAM,
                    MARAUDER_ENCAMPMENT_SUB_PARAM,
                    MOTTE_SUB_PARAM,
                ]
            },
        })
        .projection(doc! {
            "_id": 0,
            "body.content.level": 1,
            "body.content.tier": 1,
            "body.content.percentage": 1,
            "body.sub_param": 1,
            "rewards": 1,
        })
        .await?;

    let mut aggregate = target_catalog()
        .into_iter()
        .map(|target| (target.key(), LevelStats::default()))
        .collect::<BTreeMap<_, _>>();
    let mut stats = BarbarianFortPrecomputeStats::default();

    while let Some(next) = cursor.next().await {
        stats.documents_read += 1;
        let document = next?;
        if accumulate_report_document(&document, &mut aggregate) {
            stats.reports_counted += 1;
        }
    }

    Ok((aggregate, stats))
}

fn accumulate_report_document(
    document: &Document,
    aggregate: &mut BTreeMap<TargetKey, LevelStats>,
) -> bool {
    let Some(sub_param) = nested_i32(document, &["body", "sub_param"]) else {
        return false;
    };
    let Some(kind) = TargetKind::from_sub_param(sub_param) else {
        return false;
    };
    let Some(level) = nested_i32(document, &["body", "content", "level"]) else {
        return false;
    };
    let key = TargetKey { kind, level };
    let Some(level_stats) = aggregate.get_mut(&key) else {
        return false;
    };

    level_stats.reports_seen += 1;

    let Some(tier) = nested_i32(document, &["body", "content", "tier"]) else {
        return true;
    };

    let damage_percentage = nested_f64(document, &["body", "content", "percentage"]);
    let tier_stats = level_stats.tiers.entry(tier).or_default();
    tier_stats.seen += 1;
    if let Some(percentage) = damage_percentage {
        tier_stats.damage_percentage.record(percentage);
    }

    if let Some(Bson::Array(rewards)) = nested_bson(document, &["rewards"]) {
        for reward in rewards {
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

            tier_stats.loot.entry(LootKey { reward_type, sub_type }).or_default().record(quantity);
        }
    }

    true
}

fn build_precomputed_document(
    target: TargetMetadata,
    stats: &LevelStats,
    refreshed_at: DateTime,
) -> Document {
    let reports_seen = usize_to_i64(stats.reports_seen);
    let tiers = stats
        .tiers
        .iter()
        .map(|(tier, tier_stats)| build_tier_document(*tier, tier_stats, stats.reports_seen))
        .collect::<Vec<_>>();

    doc! {
        "kind": target.kind.sub_param(),
        "level": target.level,
        "reward_tiers": tiers,
        "data": {
            "ap_cost": target.ap_cost,
            "honor_points": target.honor_points,
        },
        "totals": {
            "results": reports_seen,
            "ap_used": reports_seen * i64::from(target.ap_cost),
            "honor_points_gained": reports_seen * i64::from(target.honor_points),
        },
        "refreshed_at": refreshed_at,
    }
}

fn build_tier_document(tier: i32, stats: &TierStats, level_reports_seen: usize) -> Document {
    let tier_seen = usize_to_i64(stats.seen);
    let loot = stats
        .loot
        .iter()
        .map(|(key, loot_stats)| build_loot_document(*key, loot_stats, stats.seen))
        .collect::<Vec<_>>();

    doc! {
        "tier": tier,
        "results": tier_seen,
        "receive_rate": rate(stats.seen, level_reports_seen),
        "damage_percentage": stats.damage_percentage.to_document(),
        "loot": loot,
    }
}

fn build_loot_document(key: LootKey, stats: &LootStats, tier_seen: usize) -> Document {
    doc! {
        "type": key.reward_type,
        "sub_type": key.sub_type,
        "results": usize_to_i64(stats.seen),
        "drop_rate": rate(stats.seen, tier_seen),
        "quantity": {
            "min": stats.quantity.min.unwrap_or(0),
            "max": stats.quantity.max.unwrap_or(0),
        },
        "total_quantity": stats.total_quantity,
        "average_quantity": rate_i64(stats.total_quantity, stats.seen),
    }
}

fn target_catalog() -> Vec<TargetMetadata> {
    let mut targets = Vec::with_capacity(22);

    for level in 1..=15 {
        targets.push(TargetMetadata {
            kind: TargetKind::BarbarianFort,
            level,
            ap_cost: if level >= 11 { 300 } else { 150 },
            honor_points: honor_points_for_level(level),
        });
    }

    targets.push(TargetMetadata {
        kind: TargetKind::MarauderEncampment,
        level: 1,
        ap_cost: 150,
        honor_points: 0,
    });
    targets.push(TargetMetadata {
        kind: TargetKind::MarauderEncampment,
        level: 11,
        ap_cost: 300,
        honor_points: 45,
    });

    for level in 11..=15 {
        targets.push(TargetMetadata {
            kind: TargetKind::Motte,
            level,
            ap_cost: 300,
            honor_points: honor_points_for_level(level),
        });
    }

    targets
}

fn honor_points_for_level(level: i32) -> i32 {
    match level {
        11 => 30,
        12 => 45,
        13 => 60,
        14 => 80,
        15 => 100,
        _ => 0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TargetKind {
    BarbarianFort,
    MarauderEncampment,
    Motte,
}

impl TargetKind {
    fn from_sub_param(sub_param: i32) -> Option<Self> {
        match sub_param {
            BARBARIAN_FORT_SUB_PARAM => Some(Self::BarbarianFort),
            MARAUDER_ENCAMPMENT_SUB_PARAM => Some(Self::MarauderEncampment),
            MOTTE_SUB_PARAM => Some(Self::Motte),
            _ => None,
        }
    }

    fn sub_param(self) -> i32 {
        match self {
            Self::BarbarianFort => BARBARIAN_FORT_SUB_PARAM,
            Self::MarauderEncampment => MARAUDER_ENCAMPMENT_SUB_PARAM,
            Self::Motte => MOTTE_SUB_PARAM,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TargetMetadata {
    kind: TargetKind,
    level: i32,
    ap_cost: i32,
    honor_points: i32,
}

impl TargetMetadata {
    fn key(self) -> TargetKey {
        TargetKey { kind: self.kind, level: self.level }
    }

    fn selector(self) -> Document {
        doc! {
            "kind": self.kind.sub_param(),
            "level": self.level,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TargetKey {
    kind: TargetKind,
    level: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct LootKey {
    reward_type: i32,
    sub_type: i32,
}

#[derive(Debug, Default)]
struct LevelStats {
    reports_seen: usize,
    tiers: BTreeMap<i32, TierStats>,
}

#[derive(Debug, Default)]
struct TierStats {
    seen: usize,
    damage_percentage: NumericRange,
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

fn nested_i32(document: &Document, path: &[&str]) -> Option<i32> {
    nested_bson(document, path).and_then(bson_to_i32)
}

fn nested_f64(document: &Document, path: &[&str]) -> Option<f64> {
    nested_bson(document, path).and_then(bson_to_f64)
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

fn bson_to_f64(value: &Bson) -> Option<f64> {
    match value {
        Bson::Int32(value) => Some(f64::from(*value)),
        Bson::Int64(value) => Some(*value as f64),
        Bson::Double(value) if value.is_finite() => Some(*value),
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

fn optional_f64(value: Option<f64>) -> Bson {
    value.map(Bson::Double).unwrap_or(Bson::Null)
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{Bson, doc};

    use super::*;

    #[test]
    fn target_catalog_contains_expected_levels() {
        let targets = target_catalog();

        assert_eq!(targets.len(), 22);
        assert!(targets.iter().any(|target| {
            target.kind == TargetKind::BarbarianFort
                && target.level == 15
                && target.ap_cost == 300
                && target.honor_points == 100
        }));
        assert!(targets.iter().any(|target| {
            target.kind == TargetKind::MarauderEncampment
                && target.level == 11
                && target.ap_cost == 300
                && target.honor_points == 45
        }));
        assert!(targets.iter().any(|target| {
            target.kind == TargetKind::Motte
                && target.level == 11
                && target.ap_cost == 300
                && target.honor_points == 30
        }));
    }

    #[test]
    fn accumulate_report_document_groups_tiers_and_loot_by_kind_and_level() {
        let mut aggregate = target_catalog()
            .into_iter()
            .map(|target| (target.key(), LevelStats::default()))
            .collect::<BTreeMap<_, _>>();
        let report = doc! {
            "body": {
                "sub_param": 1,
                "content": {
                    "level": 7,
                    "tier": 6,
                    "percentage": 52.0,
                },
            },
            "rewards": [
                { "type": 2, "sub_type": 7006, "value": 55 },
                { "type": 2, "sub_type": 109, "value": 6 },
            ],
        };

        assert!(accumulate_report_document(&report, &mut aggregate));

        let stats = aggregate
            .get(&TargetKey { kind: TargetKind::BarbarianFort, level: 7 })
            .expect("level stats");
        assert_eq!(stats.reports_seen, 1);
        let tier = stats.tiers.get(&6).expect("tier stats");
        assert_eq!(tier.damage_percentage.min, Some(52.0));
        assert_eq!(tier.loot.len(), 2);
    }

    #[test]
    fn build_precomputed_document_calculates_rates_and_totals() {
        let target = TargetMetadata {
            kind: TargetKind::BarbarianFort,
            level: 7,
            ap_cost: 150,
            honor_points: 0,
        };
        let mut stats = LevelStats { reports_seen: 2, ..LevelStats::default() };
        let tier = stats.tiers.entry(6).or_default();
        tier.seen = 2;
        tier.damage_percentage.record(49.0);
        tier.damage_percentage.record(52.0);
        tier.loot.entry(LootKey { reward_type: 2, sub_type: 7006 }).or_default().record(55);

        let document = build_precomputed_document(target, &stats, DateTime::now());

        assert_eq!(document.get_i32("kind"), Ok(1));
        assert_eq!(
            document.get_document("totals").and_then(|totals| totals.get_i64("results")),
            Ok(2)
        );
        assert_eq!(document.get_document("data").and_then(|data| data.get_i32("ap_cost")), Ok(150));
        assert_eq!(
            document.get_document("totals").and_then(|totals| totals.get_i64("ap_used")),
            Ok(300)
        );
        let tiers = document.get_array("reward_tiers").expect("reward tiers");
        let tier = tiers.first().and_then(Bson::as_document).expect("tier");
        assert_eq!(tier.get_i64("results"), Ok(2));
        assert_eq!(tier.get_f64("receive_rate"), Ok(1.0));
        assert!(!tier.contains_key("receive_percent"));
        let loot = tier.get_array("loot").expect("loot");
        let item = loot.first().and_then(Bson::as_document).expect("item");
        assert_eq!(item.get_i64("results"), Ok(1));
        assert_eq!(item.get_f64("drop_rate"), Ok(0.5));
        assert!(!item.contains_key("drop_percent"));
    }

    #[test]
    fn accumulate_report_document_ignores_unknown_target_levels() {
        let mut aggregate = BTreeMap::new();
        let report = doc! {
            "body": {
                "sub_param": 3,
                "content": { "level": 12, "tier": 1 },
            },
        };

        assert!(!accumulate_report_document(&report, &mut aggregate));
    }
}
