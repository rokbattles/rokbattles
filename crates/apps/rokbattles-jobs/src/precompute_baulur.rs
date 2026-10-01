//! Precompute Baulur reward rolls without confusing collapsed display positions with slots.

use std::collections::{BTreeMap, VecDeque};

use futures::StreamExt;
use mongodb::{
    Collection,
    bson::{Bson, DateTime, Document, doc},
};
use rokbattles_api::db::ReportsStore;
use rokbattles_bson::{
    bson_to_f64, bson_to_i32_exact as bson_to_i32, bson_to_i64_exact as bson_to_i64,
};
use rustc_hash::FxHashSet;
use serde::ser::Error as _;

use crate::error::JobsError;

const BAULUR_TARGET_KINDS: [i32; 2] = [102_000_055, 102_000_063];
const ROLL_SLOTS: usize = 5;
// Recipient copies observed in the source arrive milliseconds apart. Match complete
// outcomes within two seconds; never merge identical rewards from later kills.
const DUPLICATE_WINDOW_MICROS: i64 = 2_000_000;
const SLOT_ITEMS: [&[i32]; ROLL_SLOTS] =
    [&[26], &[60, 65, 85], &[47, 48], &[7051, 7052, 7053], &[113, 115, 124, 9998]];

/// Counts from one Baulur precompute run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BaulurPrecomputeStats {
    pub documents_read: usize,
    pub duplicate_reports_skipped: usize,
    pub results_counted: usize,
    pub unmatched_results: usize,
    pub documents_written: usize,
}

/// Refresh Baulur precomputed reward documents.
pub async fn precompute_baulur_data(
    reports_store: &ReportsStore,
) -> Result<BaulurPrecomputeStats, JobsError> {
    let (documents, mut stats) = compute_baulur_documents(reports_store).await?;
    let precomputed = reports_store.precomputed_baulur_collection();
    for document in documents {
        let kind = document.get_i32("kind").map_err(mongodb::bson::ser::Error::custom)?;
        precomputed.replace_one(doc! { "kind": kind }, document).upsert(true).await?;
        stats.documents_written += 1;
    }
    Ok(stats)
}

/// Compute the exact scheduled-job output using only database reads.
/// This path does not refresh documents or create indexes.
pub async fn compute_baulur_documents(
    reports_store: &ReportsStore,
) -> Result<(Vec<Document>, BaulurPrecomputeStats), JobsError> {
    let (aggregate, stats) =
        read_observed_baulur_reports(reports_store.barcanyonkillboss_collection()).await?;
    let refreshed_at = DateTime::now();
    let documents = aggregate
        .iter()
        .map(|(&kind, kind_stats)| build_precomputed_document(kind, kind_stats, refreshed_at))
        .collect();
    Ok((documents, stats))
}

async fn read_observed_baulur_reports(
    source: &Collection<Document>,
) -> Result<(BTreeMap<i32, KindStats>, BaulurPrecomputeStats), JobsError> {
    let mut cursor = source
        .aggregate([
            doc! { "$match": { "npc.type": { "$in": BAULUR_TARGET_KINDS.to_vec() } } },
            doc! { "$project": {
                "_id": 0,
                "metadata.mail_time": 1,
                "metadata.server_id": 1,
                "npc.type": 1,
                "npc.location": 1,
                "participants.player_id": 1,
                "participants.damage_rate": 1,
                "participants.loot": 1,
            } },
            doc! { "$sort": { "metadata.mail_time": 1 } },
        ])
        .allow_disk_use(true)
        .await?;
    let mut aggregate: BTreeMap<i32, KindStats> =
        BAULUR_TARGET_KINDS.into_iter().map(|kind| (kind, KindStats::default())).collect();
    let mut stats = BaulurPrecomputeStats::default();
    let mut recent = RecentReports::default();
    while let Some(next) = cursor.next().await {
        let document = next?;
        stats.documents_read += 1;
        let Some(kind_stats) =
            nested_i32(&document, &["npc", "type"]).and_then(|kind| aggregate.get_mut(&kind))
        else {
            continue;
        };
        kind_stats.reports += 1;
        if recent.is_duplicate(&document) {
            stats.duplicate_reports_skipped += 1;
            kind_stats.duplicate_reports += 1;
            continue;
        }
        stats.results_counted += accumulate_report_document(&document, kind_stats);
    }
    stats.unmatched_results = aggregate.values().map(|kind| kind.roll_pool.unmatched_results).sum();
    Ok((aggregate, stats))
}

fn accumulate_report_document(document: &Document, stats: &mut KindStats) -> usize {
    let Ok(participants) = document.get_array("participants") else {
        return 0;
    };
    let mut counted = 0;
    for participant in participants.iter().filter_map(Bson::as_document) {
        let Some(damage) = direct_f64(participant, "damage_rate") else {
            continue;
        };
        if !damage.is_finite() || !(0.0..=100.0).contains(&damage) {
            continue;
        }
        stats.results += 1;
        counted += 1;
        if damage <= 1.0 {
            let pool = &mut stats.resource_pool;
            pool.results += 1;
            pool.damage_rate.record(damage);
            if let Ok(loot) = participant.get_array("loot") {
                for reward in loot.iter().filter_map(Reward::parse) {
                    pool.loot.entry(reward.key).or_default().record(reward.quantity);
                }
            }
        } else {
            let pool = &mut stats.roll_pool;
            pool.results += 1;
            pool.damage_rate.record(damage);
            let Some(rolls) = reconstruct_rolls(participant) else {
                pool.unmatched_results += 1;
                continue;
            };
            for (slot, reward) in pool.slots.iter_mut().zip(rolls) {
                if let Some(reward) = reward {
                    slot.results += 1;
                    slot.loot.entry(reward.key).or_default().record(reward.quantity);
                }
            }
        }
    }
    counted
}

fn reconstruct_rolls(participant: &Document) -> Option<[Option<Reward>; ROLL_SLOTS]> {
    let mut rolls = [None; ROLL_SLOTS];
    let mut previous_slot = None;
    for reward in participant.get_array("loot").ok()? {
        let reward = Reward::parse(reward)?;
        let slot = slot_for_item(reward.key)?;
        if previous_slot.is_some_and(|previous| slot <= previous) {
            return None;
        }
        *rolls.get_mut(slot)? = Some(reward);
        previous_slot = Some(slot);
    }
    Some(rolls)
}

fn slot_for_item(key: LootKey) -> Option<usize> {
    if key.reward_type != 2 {
        return None;
    }
    SLOT_ITEMS.iter().position(|items| items.contains(&key.sub_type))
}

fn build_precomputed_document(kind: i32, stats: &KindStats, refreshed_at: DateTime) -> Document {
    doc! {
        "kind": kind,
        "resource_pool": {
            "results": usize_to_i64(stats.resource_pool.results),
            "damage_factor": stats.resource_pool.damage_rate.to_document(),
            "loot": stats.resource_pool.loot.iter()
                .map(|(&key, loot)| build_loot_document(key, loot, stats.resource_pool.results, 0))
                .collect::<Vec<_>>(),
        },
        "roll_pool": build_roll_pool_document(&stats.roll_pool),
        "totals": {
            "results": usize_to_i64(stats.results),
            "reports": usize_to_i64(stats.reports),
            "duplicate_reports": usize_to_i64(stats.duplicate_reports),
        },
        "refreshed_at": refreshed_at,
    }
}

fn build_roll_pool_document(stats: &RollPoolStats) -> Document {
    let matched = stats.results - stats.unmatched_results;
    let slots = stats
        .slots
        .iter()
        .zip(SLOT_ITEMS)
        .enumerate()
        .map(|(index, (slot, items))| {
            let loot = items
                .iter()
                .map(|&sub_type| {
                    let key = LootKey { reward_type: 2, sub_type };
                    let empty = LootStats::default();
                    let quantity = match index {
                        0 => 3,
                        1 => 2,
                        _ => 1,
                    };
                    build_loot_document(
                        key,
                        slot.loot.get(&key).unwrap_or(&empty),
                        matched,
                        quantity,
                    )
                })
                .collect::<Vec<_>>();
            doc! {
                "slot": usize_to_i64(index + 1),
                "results": usize_to_i64(slot.results),
                "no_item_results": usize_to_i64(matched - slot.results),
                "no_item_rate": rate(matched - slot.results, matched),
                "loot": loot,
            }
        })
        .collect::<Vec<_>>();
    doc! {
        "results": usize_to_i64(stats.results),
        "matched_results": usize_to_i64(matched),
        "unmatched_results": usize_to_i64(stats.unmatched_results),
        "damage_factor": stats.damage_rate.to_document(),
        "slots": slots,
    }
}

fn build_loot_document(
    key: LootKey,
    stats: &LootStats,
    results: usize,
    default_quantity: i64,
) -> Document {
    doc! {
        "type": key.reward_type,
        "sub_type": key.sub_type,
        "results": usize_to_i64(stats.seen),
        "drop_rate": rate(stats.seen, results),
        "quantity": {
            "min": stats.quantity.min.unwrap_or(default_quantity),
            "max": stats.quantity.max.unwrap_or(default_quantity),
        },
        "total_quantity": stats.total_quantity,
        "average_quantity": rate_i64(stats.total_quantity, stats.seen),
    }
}

#[derive(Debug, Default)]
struct KindStats {
    results: usize,
    reports: usize,
    duplicate_reports: usize,
    resource_pool: PoolStats,
    roll_pool: RollPoolStats,
}

#[derive(Debug, Default)]
struct PoolStats {
    results: usize,
    damage_rate: NumericRange,
    loot: BTreeMap<LootKey, LootStats>,
}

#[derive(Debug, Default)]
struct RollPoolStats {
    results: usize,
    unmatched_results: usize,
    damage_rate: NumericRange,
    slots: [SlotStats; ROLL_SLOTS],
}

#[derive(Debug, Default)]
struct SlotStats {
    results: usize,
    loot: BTreeMap<LootKey, LootStats>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct LootKey {
    reward_type: i32,
    sub_type: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ParticipantOutcome {
    player_id: i64,
    damage: u64,
    loot: Vec<Reward>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ReportKey {
    kind: i32,
    server_id: i64,
    location: [u64; 2],
    participants: Vec<ParticipantOutcome>,
}

impl ReportKey {
    fn parse(document: &Document) -> Option<Self> {
        let metadata = document.get_document("metadata").ok()?;
        let npc = document.get_document("npc").ok()?;
        let location = npc.get_document("location").ok()?;
        let mut participants = document
            .get_array("participants")
            .ok()?
            .iter()
            .map(|value| {
                let participant = value.as_document()?;
                Some(ParticipantOutcome {
                    player_id: direct_i64(participant, "player_id")?,
                    damage: direct_f64(participant, "damage_rate")?.to_bits(),
                    loot: participant
                        .get_array("loot")
                        .ok()?
                        .iter()
                        .map(Reward::parse)
                        .collect::<Option<_>>()?,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        if participants.is_empty() {
            return None;
        }
        participants.sort_unstable();
        Some(Self {
            kind: direct_i32(npc, "type")?,
            server_id: direct_i64(metadata, "server_id")?,
            location: [direct_f64(location, "x")?.to_bits(), direct_f64(location, "y")?.to_bits()],
            participants,
        })
    }
}

#[derive(Debug, Default)]
struct RecentReports {
    keys: FxHashSet<ReportKey>,
    arrivals: VecDeque<(i64, ReportKey)>,
}

impl RecentReports {
    fn is_duplicate(&mut self, document: &Document) -> bool {
        let Some(time) = nested_bson(document, &["metadata", "mail_time"]).and_then(bson_to_i64)
        else {
            return false;
        };
        while self
            .arrivals
            .front()
            .is_some_and(|(first, _)| time.saturating_sub(*first) > DUPLICATE_WINDOW_MICROS)
        {
            if let Some((_, key)) = self.arrivals.pop_front() {
                self.keys.remove(&key);
            }
        }
        let Some(key) = ReportKey::parse(document) else {
            return false;
        };
        if !self.keys.insert(key.clone()) {
            return true;
        }
        self.arrivals.push_back((time, key));
        false
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
        doc! { "min": optional_f64(self.min), "max": optional_f64(self.max) }
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
    use super::*;

    fn participant(damage: f64, items: &[i32]) -> Document {
        doc! {
            "player_id": 42_i64,
            "damage_rate": damage,
            "loot": items.iter().map(|&sub_type| doc! {
                "type": 2, "sub_type": sub_type,
                "value": match sub_type { 26 => 3, 60 | 65 | 85 => 2, _ => 1 },
            }).collect::<Vec<_>>(),
        }
    }
    fn report(time: i64, participants: Vec<Document>) -> Document {
        doc! {
            "metadata": { "mail_time": time, "server_id": 1804, "mail_receiver": "recipient" },
            "npc": { "type": 102_000_055, "location": { "x": 100.0, "y": 200.0 } },
            "participants": participants,
        }
    }
    fn regular_pool(document: &Document) -> &Document {
        document.get_document("roll_pool").expect("roll pool")
    }
    fn slots(document: &Document) -> Vec<&Document> {
        regular_pool(document)
            .get_array("slots")
            .expect("slots")
            .iter()
            .map(|value| value.as_document().expect("slot"))
            .collect()
    }

    #[test]
    fn exactly_one_percent_remains_a_resource_pack_result() {
        let mut stats = KindStats::default();
        accumulate_report_document(&report(0, vec![participant(1.0, &[7006])]), &mut stats);
        assert_eq!((stats.resource_pool.results, stats.roll_pool.results), (1, 0));
    }

    #[test]
    fn collapsed_rewards_keep_their_original_slots() {
        let mut stats = KindStats::default();
        accumulate_report_document(&report(0, vec![participant(12.5, &[26, 65, 115])]), &mut stats);
        let document = build_precomputed_document(102_000_055, &stats, DateTime::now());
        let actual = slots(&document)
            .iter()
            .map(|slot| {
                (
                    slot.get_i64("results").expect("results"),
                    slot.get_i64("no_item_results").expect("none"),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, vec![(1, 0), (1, 0), (0, 1), (0, 1), (1, 0)]);
    }

    #[test]
    fn item_and_no_item_rates_use_all_matched_results() {
        let mut stats = KindStats::default();
        accumulate_report_document(
            &report(
                0,
                vec![participant(5.0, &[26, 60]), participant(5.0, &[26, 85, 48, 7053, 9998])],
            ),
            &mut stats,
        );
        let document = build_precomputed_document(102_000_055, &stats, DateTime::now());
        let slots = slots(&document);
        let third = slots.get(2).expect("third slot");
        let golden_key = third
            .get_array("loot")
            .expect("loot")
            .iter()
            .filter_map(Bson::as_document)
            .find(|loot| loot.get_i32("sub_type") == Ok(48))
            .expect("golden key");
        assert_eq!(golden_key.get_f64("drop_rate"), Ok(0.5));
        assert_eq!(third.get_f64("no_item_rate"), Ok(0.5));
        assert!(!third.contains_key("receive_rate"));
        assert!(!regular_pool(&document).contains_key("item_counts"));
    }

    #[test]
    fn unknown_or_out_of_order_rewards_do_not_turn_into_empty_rolls() {
        let mut stats = KindStats::default();
        accumulate_report_document(
            &report(
                0,
                vec![
                    participant(5.0, &[26, 60]),
                    participant(5.0, &[26, 60, 7006]),
                    participant(5.0, &[26, 60, 85]),
                    participant(5.0, &[26, 115, 47]),
                ],
            ),
            &mut stats,
        );
        let document = build_precomputed_document(102_000_055, &stats, DateTime::now());
        assert_eq!(
            (
                regular_pool(&document).get_i64("matched_results").expect("matched"),
                regular_pool(&document).get_i64("unmatched_results").expect("unmatched"),
                slots(&document).first().expect("first").get_i64("no_item_results").expect("none")
            ),
            (1, 3, 0)
        );
    }

    #[test]
    fn empty_loot_is_five_empty_rolls_but_missing_loot_is_unmatched() {
        let mut missing = participant(5.0, &[]);
        missing.remove("loot");
        let mut stats = KindStats::default();
        accumulate_report_document(&report(0, vec![participant(5.0, &[]), missing]), &mut stats);
        let document = build_precomputed_document(102_000_055, &stats, DateTime::now());
        assert_eq!(
            (
                regular_pool(&document).get_i64("matched_results").expect("matched"),
                regular_pool(&document).get_i64("unmatched_results").expect("unmatched")
            ),
            (1, 1)
        );
    }

    #[test]
    fn recipient_copies_across_a_second_boundary_are_deduplicated() {
        let first = report(999_990, vec![participant(5.0, &[26, 60])]);
        let mut second = report(1_000_020, vec![participant(5.0, &[26, 60])]);
        second
            .get_document_mut("metadata")
            .expect("metadata")
            .insert("mail_receiver", "another recipient");
        let mut recent = RecentReports::default();
        assert!(!recent.is_duplicate(&first));
        assert!(recent.is_duplicate(&second));
    }

    #[test]
    fn participant_order_does_not_prevent_deduplication() {
        let a = participant(5.0, &[26, 60]);
        let mut b = participant(95.0, &[26, 85, 48]);
        b.insert("player_id", 43_i64);
        let mut recent = RecentReports::default();
        assert!(!recent.is_duplicate(&report(0, vec![a.clone(), b.clone()])));
        assert!(recent.is_duplicate(&report(30, vec![b, a])));
    }

    #[test]
    fn later_kills_and_different_outcomes_are_counted_separately() {
        let mut recent = RecentReports::default();
        assert!(!recent.is_duplicate(&report(0, vec![participant(5.0, &[26, 60])])));
        assert!(!recent.is_duplicate(&report(30, vec![participant(5.0, &[26, 65])])));
        assert!(
            !recent.is_duplicate(&report(
                DUPLICATE_WINDOW_MICROS + 1,
                vec![participant(5.0, &[26, 60])]
            ))
        );
    }

    #[test]
    fn missing_battle_identity_does_not_discard_reports() {
        let mut document = report(0, vec![participant(5.0, &[26, 60])]);
        document.remove("metadata");
        let mut recent = RecentReports::default();
        assert!(!recent.is_duplicate(&document));
        assert!(!recent.is_duplicate(&document));
    }

    #[test]
    fn empty_dataset_has_five_slots_and_finite_zero_rates() {
        let document =
            build_precomputed_document(102_000_063, &KindStats::default(), DateTime::now());
        assert_eq!(
            slots(&document)
                .iter()
                .map(|slot| slot.get_f64("no_item_rate").expect("rate"))
                .collect::<Vec<_>>(),
            vec![0.0; 5]
        );
    }
}
