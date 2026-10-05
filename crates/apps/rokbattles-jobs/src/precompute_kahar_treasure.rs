//! Precompute Kahar treasure's five ordered reward slots from individual reward mails.

use std::collections::BTreeMap;

use futures::StreamExt;
use mongodb::{
    Collection,
    bson::{Bson, DateTime, Document, doc},
};
use rokbattles_api::db::ReportsStore;
use rokbattles_bson::{bson_to_i32_exact as bson_to_i32, bson_to_i64_exact as bson_to_i64};

use crate::error::JobsError;

const KAHAR_TREASURE_AGGREGATE_KEY: &str = "all";
const KAHAR_AP_COST: i64 = 200;
const REWARD_SLOTS: usize = 5;

/// Counts from one Kahar treasure precompute run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KaharTreasurePrecomputeStats {
    pub documents_read: usize,
    pub mails_counted: usize,
    pub empty_results_skipped: usize,
    pub unmatched_results: usize,
    pub documents_written: usize,
}

/// Refresh Kahar treasure precomputed reward documents.
pub async fn precompute_kahar_treasure_data(
    reports_store: &ReportsStore,
) -> Result<KaharTreasurePrecomputeStats, JobsError> {
    let (document, mut stats) = compute_kahar_treasure_document(reports_store).await?;

    let precomputed = reports_store.precomputed_kahar_treasure_collection();

    precomputed
        .replace_one(doc! { "kind": KAHAR_TREASURE_AGGREGATE_KEY }, document)
        .upsert(true)
        .await?;

    stats.documents_written += 1;

    Ok(stats)
}

pub async fn compute_kahar_treasure_document(
    reports_store: &ReportsStore,
) -> Result<(Document, KaharTreasurePrecomputeStats), JobsError> {
    let (aggregate, stats) =
        read_observed_kahar_treasure_reports(reports_store.system_kahar_treasure_collection())
            .await?;

    let document = build_precomputed_document(&aggregate, DateTime::now());

    Ok((document, stats))
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
        let document = next?;
        stats.documents_read += 1;

        accumulate_report_document(&document, &mut aggregate);
    }

    stats.mails_counted = aggregate.results;
    stats.empty_results_skipped = aggregate.empty_results;
    stats.unmatched_results = aggregate.unmatched_results;

    Ok((aggregate, stats))
}

fn accumulate_report_document(document: &Document, aggregate: &mut AggregateStats) {
    let loot = document.get_array("loot").ok();

    if loot.is_some_and(Vec::is_empty) {
        aggregate.empty_results += 1;

        return;
    }

    let Some(rewards) = loot.and_then(|loot| parse_slots(loot)) else {
        aggregate.unmatched_results += 1;

        return;
    };

    aggregate.results += 1;

    for (slot, reward) in aggregate.slots.iter_mut().zip(rewards) {
        slot.entry(reward.key).or_default().record(reward.quantity);
    }
}

fn parse_slots(loot: &[Bson]) -> Option<[Reward; REWARD_SLOTS]> {
    if loot.len() != REWARD_SLOTS {
        return None;
    }

    loot.iter().map(Reward::parse).collect::<Option<Vec<_>>>()?.try_into().ok()
}

fn build_precomputed_document(stats: &AggregateStats, refreshed_at: DateTime) -> Document {
    let slots = stats
        .slots
        .iter()
        .enumerate()
        .map(|(index, slot)| {
            let loot = slot
                .iter()
                .map(|(key, loot_stats)| build_loot_document(*key, loot_stats, stats.results))
                .collect::<Vec<_>>();

            doc! {
                "slot": usize_to_i64(index + 1),
                "results": usize_to_i64(stats.results),
                "loot": loot,
            }
        })
        .collect::<Vec<_>>();

    doc! {
        "kind": KAHAR_TREASURE_AGGREGATE_KEY,
        "slots": slots,
        "totals": {
            "results": usize_to_i64(stats.results),
            "ap_used": usize_to_i64(stats.results).saturating_mul(KAHAR_AP_COST),
            "empty_results": usize_to_i64(stats.empty_results),
            "unmatched_results": usize_to_i64(stats.unmatched_results),
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
    empty_results: usize,
    unmatched_results: usize,
    slots: [BTreeMap<LootKey, LootStats>; REWARD_SLOTS],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct LootKey {
    reward_type: i32,
    sub_type: i32,
}

#[derive(Debug, Clone, Copy)]
struct Reward {
    key: LootKey,
    quantity: i64,
}

impl Reward {
    fn parse(value: &Bson) -> Option<Self> {
        let document = value.as_document()?;
        let quantity = direct_i64(document, "value")?;

        if quantity <= 0 {
            return None;
        }

        Some(Self {
            key: LootKey {
                reward_type: direct_i32(document, "type")?,
                sub_type: direct_i32(document, "sub_type")?,
            },
            quantity,
        })
    }
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
    use super::*;

    fn report(crystals: i64, fourth: i32, fifth: i32) -> Document {
        doc! {
            "loot": [
                { "type": 1, "sub_type": 9, "value": crystals },
                { "type": 2, "sub_type": 147, "value": 5 },
                { "type": 2, "sub_type": 10, "value": 1 },
                { "type": 2, "sub_type": fourth, "value": 1 },
                { "type": 2, "sub_type": fifth, "value": 1 },
            ],
        }
    }

    #[test]
    fn identical_individual_rewards_each_count_as_a_result() {
        let mut aggregate = AggregateStats::default();
        let report = report(45_000, 74, 94);

        accumulate_report_document(&report, &mut aggregate);
        accumulate_report_document(&report, &mut aggregate);

        assert_eq!(aggregate.results, 2);

        for slot in &aggregate.slots {
            assert_eq!(slot.len(), 1);
            assert_eq!(slot.values().next().expect("reward").seen, 2);
        }
    }

    #[test]
    fn repeated_items_in_last_two_slots_remain_separate() {
        let mut aggregate = AggregateStats::default();

        accumulate_report_document(&report(45_000, 74, 74), &mut aggregate);
        accumulate_report_document(&report(45_000, 74, 94), &mut aggregate);

        let document = build_precomputed_document(&aggregate, DateTime::now());
        let slots = document.get_array("slots").expect("slots");
        let fourth = slots[3].as_document().expect("fourth slot");
        let fifth = slots[4].as_document().expect("fifth slot");
        let fourth_loot = fourth.get_array("loot").expect("fourth loot");
        let fifth_loot = fifth.get_array("loot").expect("fifth loot");

        assert_eq!(fourth_loot.len(), 1);
        assert_eq!(fifth_loot.len(), 2);

        let fourth_item = fourth_loot[0].as_document().expect("fourth reward");
        let fifth_item = fifth_loot[0].as_document().expect("fifth reward");

        assert_eq!(fourth_item.get_i32("sub_type"), Ok(74));
        assert_eq!(fifth_item.get_i32("sub_type"), Ok(74));

        assert_eq!(fourth_item.get_f64("drop_rate"), Ok(1.0));
        assert_eq!(fifth_item.get_f64("drop_rate"), Ok(0.5));

        assert_eq!(fourth_item.get_i64("total_quantity"), Ok(2));
        assert_eq!(fifth_item.get_i64("total_quantity"), Ok(1));
    }

    #[test]
    fn historical_crystal_quantities_preserve_ranges_and_totals() {
        let mut aggregate = AggregateStats::default();

        accumulate_report_document(&report(30_000, 74, 94), &mut aggregate);
        accumulate_report_document(&report(90_000, 74, 94), &mut aggregate);

        let document = build_precomputed_document(&aggregate, DateTime::now());
        let slots = document.get_array("slots").expect("slots");

        assert_eq!(slots.len(), REWARD_SLOTS);

        for (index, slot) in slots.iter().enumerate() {
            let slot = slot.as_document().expect("slot");

            assert_eq!(slot.get_i64("slot"), Ok(index as i64 + 1));
            assert_eq!(slot.get_i64("results"), Ok(2));
        }

        let loot = slots[0].as_document().expect("first slot").get_array("loot").expect("loot");
        let crystals = loot[0].as_document().expect("crystals");

        assert_eq!(crystals.get_f64("drop_rate"), Ok(1.0));
        assert_eq!(crystals.get_i64("total_quantity"), Ok(120_000));
        assert_eq!(crystals.get_f64("average_quantity"), Ok(60_000.0));
        assert_eq!(
            crystals.get_document("quantity"),
            Ok(&doc! { "min": 30_000_i64, "max": 90_000_i64 })
        );

        assert_eq!(document.get_document("totals").expect("totals").get_i64("ap_used"), Ok(400));
        assert!(!document.contains_key("loot"));
    }

    #[test]
    fn empty_rewards_do_not_change_slot_rates_or_ap_totals() {
        let mut aggregate = AggregateStats::default();

        accumulate_report_document(&doc! { "loot": [] }, &mut aggregate);
        accumulate_report_document(&report(45_000, 74, 94), &mut aggregate);

        let document = build_precomputed_document(&aggregate, DateTime::now());
        let totals = document.get_document("totals").expect("totals");

        assert_eq!(totals.get_i64("results"), Ok(1));
        assert_eq!(totals.get_i64("ap_used"), Ok(200));
        assert_eq!(totals.get_i64("empty_results"), Ok(1));
        assert_eq!(totals.get_i64("unmatched_results"), Ok(0));

        for slot in document.get_array("slots").expect("slots") {
            let loot = slot.as_document().expect("slot").get_array("loot").expect("loot");

            assert_eq!(loot[0].as_document().expect("reward").get_f64("drop_rate"), Ok(1.0));
        }
    }

    #[test]
    fn incomplete_or_malformed_rewards_are_excluded_without_shifting_slots() {
        let mut invalid = vec![doc! {}, doc! { "loot": "invalid" }];
        let complete = report(45_000, 74, 94);
        let loot = complete.get_array("loot").expect("loot");

        invalid.push(doc! { "loot": loot[..4].to_vec() });

        let mut extra = loot.clone();
        extra.push(loot[4].clone());
        invalid.push(doc! { "loot": extra });

        for reward in [
            Bson::Null,
            Bson::Document(doc! { "type": 2, "sub_type": 94 }),
            Bson::Document(doc! { "type": 2, "sub_type": 94, "value": 0 }),
            Bson::Document(doc! { "type": 2, "sub_type": 94, "value": -1 }),
            Bson::Document(doc! { "type": 2, "sub_type": 94, "value": 1.5 }),
            Bson::Document(doc! { "type": 2, "sub_type": "94", "value": 1 }),
            Bson::Document(doc! { "type": i64::MAX, "sub_type": 94, "value": 1 }),
        ] {
            let mut malformed = loot.clone();
            malformed[4] = reward;
            invalid.push(doc! { "loot": malformed });
        }

        let mut aggregate = AggregateStats::default();

        for document in &invalid {
            accumulate_report_document(document, &mut aggregate);
        }

        assert_eq!(aggregate.results, 0);
        assert_eq!(aggregate.empty_results, 0);
        assert_eq!(aggregate.unmatched_results, invalid.len());
        assert!(aggregate.slots.iter().all(BTreeMap::is_empty));
    }

    #[test]
    fn no_results_produces_five_empty_slots() {
        let document = build_precomputed_document(&AggregateStats::default(), DateTime::now());
        let slots = document.get_array("slots").expect("slots");

        assert_eq!(slots.len(), REWARD_SLOTS);

        for slot in slots {
            let slot = slot.as_document().expect("slot");

            assert_eq!(slot.get_i64("results"), Ok(0));
            assert!(slot.get_array("loot").expect("loot").is_empty());
        }
    }
}
