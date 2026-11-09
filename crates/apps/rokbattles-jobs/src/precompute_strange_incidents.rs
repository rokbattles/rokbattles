//! Loot statistics for Strange Incidents.
//!
//! Processes level 1 bandit reports (NPC 70005) from the last 365 days.
//!
//! Rewards are grouped into six slots inferred from observed report order.
//! Reports omit empty slots, so an item's position in the reward list can vary.
//! Unknown rewards and invalid slot orders count as unmatched results and do
//! not contribute to drop rates.

use std::collections::{BTreeMap, VecDeque};

use futures::StreamExt;
use mongodb::{
    Collection,
    bson::{Bson, DateTime, Document, doc},
};
use rokbattles_api::db::ReportsStore;
use rokbattles_bson::{bson_to_i64_exact, nested_i64_exact as nested_i64};
use rustc_hash::FxHashSet;
use serde::ser::Error as _;

use crate::{error::JobsError, loot_window::loot_cutoff_mail_time};

const NPC_IDS: [i64; 1] = [70_005];
const REWARD_SLOTS: usize = 6;

// Recipient copies can have different mail timestamps. Compare complete outcomes
// within two seconds to avoid counting those copies as separate kills.
const DUPLICATE_WINDOW_MICROS: i64 = 2_000_000;

/// Counts collected while computing Strange Incidents loot statistics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StrangeIncidentsPrecomputeStats {
    /// Source reports read before deduplication.
    pub documents_read: usize,
    /// Recipient copies excluded from the results.
    pub duplicate_reports_skipped: usize,
    /// Participant results with empty reward lists, excluding report copies.
    pub empty_results_skipped: usize,
    /// Participant results that could not be assigned to reward slots.
    pub unmatched_results: usize,
    /// Participant results considered, including unmatched results.
    pub results_counted: usize,
    /// Aggregate documents successfully written to MongoDB.
    pub documents_written: usize,
}

/// Refreshes the stored Strange Incidents loot statistics.
///
/// Replaces the aggregates for supported NPCs and removes aggregates for other NPCs.
///
/// # Errors
///
/// Returns an error if a MongoDB operation fails or an output document has an invalid `kind`.
pub async fn precompute_strange_incidents_data(
    reports_store: &ReportsStore,
) -> Result<StrangeIncidentsPrecomputeStats, JobsError> {
    let (documents, mut stats) = compute_strange_incidents_documents(reports_store).await?;
    let output = reports_store.precomputed_strange_incidents_collection();

    for document in documents {
        let kind = document.get_i64("kind").map_err(mongodb::bson::ser::Error::custom)?;
        output.replace_one(doc! { "kind": kind }, document).upsert(true).await?;
        stats.documents_written += 1;
    }

    output.delete_many(doc! { "kind": { "$nin": NPC_IDS.to_vec() } }).await?;

    Ok(stats)
}

/// Computes Strange Incidents loot statistics without writing to MongoDB.
///
/// Returns the aggregate documents and computation counts. The documents share
/// a refresh timestamp, which also determines the 365-day cutoff.
///
/// # Errors
///
/// Returns an error if reading reports from MongoDB fails.
pub async fn compute_strange_incidents_documents(
    reports_store: &ReportsStore,
) -> Result<(Vec<Document>, StrangeIncidentsPrecomputeStats), JobsError> {
    let refreshed_at = DateTime::now();
    let (aggregates, stats) = read_observed_reports(
        reports_store.ghost_cav_kill_bar_collection(),
        loot_cutoff_mail_time(refreshed_at),
    )
    .await?;

    let documents = aggregates
        .iter()
        .map(|(&kind, aggregate)| build_document(kind, aggregate, refreshed_at))
        .collect();

    Ok((documents, stats))
}

async fn read_observed_reports(
    source: &Collection<Document>,
    cutoff_mail_time: i64,
) -> Result<(BTreeMap<i64, AggregateStats>, StrangeIncidentsPrecomputeStats), JobsError> {
    let mut cursor = source
        .aggregate([
            doc! { "$match": {
                "npc.type": { "$in": NPC_IDS.to_vec() },
                "npc.level": 1,
                "metadata.mail_time": { "$gte": cutoff_mail_time },
            } },
            doc! { "$project": {
                "_id": 0,
                "npc": 1,
                "metadata.mail_time": 1,
                "metadata.server_id": 1,
                "participants.player_id": 1,
                "participants.loot": 1,
                "participants.damage_rate": 1,
            } },
            doc! { "$sort": { "metadata.mail_time": 1 } },
        ])
        .allow_disk_use(true)
        .await?;

    let mut aggregates: BTreeMap<_, _> =
        NPC_IDS.into_iter().map(|id| (id, AggregateStats::default())).collect();
    let mut stats = StrangeIncidentsPrecomputeStats::default();
    let mut recent = RecentReports::default();

    while let Some(next) = cursor.next().await {
        let document = next?;
        stats.documents_read += 1;

        let Some(aggregate) =
            nested_i64(&document, &["npc", "type"]).and_then(|id| aggregates.get_mut(&id))
        else {
            continue;
        };

        aggregate.reports += 1;
        if recent.is_duplicate(&document) {
            aggregate.duplicate_reports += 1;
            stats.duplicate_reports_skipped += 1;
            continue;
        }

        accumulate_document(&document, aggregate);
    }

    for aggregate in aggregates.values() {
        stats.results_counted += aggregate.results;
        stats.empty_results_skipped += aggregate.empty_results;
        stats.unmatched_results += aggregate.unmatched_results;
    }

    Ok((aggregates, stats))
}

fn accumulate_document(document: &Document, aggregate: &mut AggregateStats) {
    let Some(kind) = nested_i64(document, &["npc", "type"]) else { return };
    let Ok(participants) = document.get_array("participants") else { return };

    for participant in participants {
        let loot = participant.as_document().and_then(|p| p.get_array("loot").ok());
        if loot.is_some_and(Vec::is_empty) {
            aggregate.empty_results += 1;
            continue;
        }

        aggregate.results += 1;
        let Some(rewards) = loot.and_then(|loot| parse_slots(kind, loot)) else {
            aggregate.unmatched_results += 1;
            continue;
        };

        aggregate.matched_results += 1;
        for (slot, reward) in aggregate.slots.iter_mut().zip(rewards) {
            if let Some(reward) = reward {
                slot.entry(reward.key).or_default().record(reward.quantity);
            }
        }
    }
}

fn parse_slots(kind: i64, loot: &[Bson]) -> Option<[Option<Reward>; REWARD_SLOTS]> {
    if !(2..=REWARD_SLOTS).contains(&loot.len()) {
        return None;
    }
    let mut slots = [None; REWARD_SLOTS];
    let mut previous_slot = None;
    for value in loot {
        let reward = Reward::parse(value)?;
        let slot = slot_for_item(kind, reward.key)?;
        if previous_slot.is_some_and(|previous| slot <= previous) {
            return None;
        }
        previous_slot = Some(slot);
        if reward.quantity > 0 {
            *slots.get_mut(slot)? = Some(reward);
        }
    }
    if slots[0].is_none() || slots[1].is_none() {
        return None;
    }
    Some(slots)
}

fn slot_for_item(kind: i64, key: LootKey) -> Option<usize> {
    if key.reward_type != 2 {
        return None;
    }
    match (kind, key.sub_type) {
        (70_005, 3 | 9 | 15 | 21 | 28) => Some(0),
        (70_005, 65 | 75 | 85 | 95) => Some(1),
        (70_005, 47 | 48 | 9998 | 9999 | 10004) => Some(2),
        (70_005, 4 | 22 | 7001) => Some(3),
        (70_005, 67 | 77 | 87 | 97) => Some(4),
        (70_005, 4257 | 8008 | 8450) => Some(5),
        _ => None,
    }
}

fn build_document(kind: i64, stats: &AggregateStats, refreshed_at: DateTime) -> Document {
    let slots = stats
        .slots
        .iter()
        .enumerate()
        .map(|(index, slot)| {
            let loot = slot
                .iter()
                .map(|(key, value)| {
                    doc! {
                        "type": key.reward_type,
                        "sub_type": key.sub_type,
                        "results": usize_to_i64(value.seen),
                        "drop_rate": rate(value.seen, stats.matched_results),
                        "quantity": { "min": value.min.unwrap_or(0), "max": value.max.unwrap_or(0) },
                        "total_quantity": value.total_quantity,
                        "average_quantity": if value.seen == 0 { 0.0 } else {
                            value.total_quantity as f64 / value.seen as f64
                        },
                    }
                })
                .collect::<Vec<_>>();

            let received: usize = slot.values().map(|value| value.seen).sum();
            doc! {
                "slot": usize_to_i64(index + 1),
                "results": usize_to_i64(stats.matched_results),
                "no_item_results": usize_to_i64(stats.matched_results - received),
                "no_item_rate": rate(stats.matched_results - received, stats.matched_results),
                "loot": loot,
            }
        })
        .collect::<Vec<_>>();

    doc! {
        "kind": kind,
        "slots": slots,
        "totals": {
            "results": usize_to_i64(stats.results),
            "matched_results": usize_to_i64(stats.matched_results),
            "reports": usize_to_i64(stats.reports),
            "duplicate_reports": usize_to_i64(stats.duplicate_reports),
            "empty_results": usize_to_i64(stats.empty_results),
            "unmatched_results": usize_to_i64(stats.unmatched_results),
        },
        "refreshed_at": refreshed_at,
    }
}

#[derive(Debug, Default)]
struct AggregateStats {
    reports: usize,
    duplicate_reports: usize,
    empty_results: usize,
    unmatched_results: usize,
    results: usize,
    matched_results: usize,
    slots: [BTreeMap<LootKey, LootStats>; REWARD_SLOTS],
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
        let key = LootKey {
            reward_type: direct_i32(document, "type")?,
            sub_type: direct_i32(document, "sub_type")?,
        };
        let empty = key.reward_type == 2 && key.sub_type == 0 && quantity == 0;
        if !empty && (quantity <= 0 || key.sub_type == 0) {
            return None;
        }
        Some(Self { key, quantity })
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
    npc_type: i64,
    npc_level: i64,
    location: [u64; 2],
    server_id: i64,
    participants: Vec<ParticipantOutcome>,
}

impl ReportKey {
    fn parse(document: &Document) -> Option<Self> {
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

        let npc = document.get_document("npc").ok()?;
        let location = npc.get_document("location").ok()?;
        Some(Self {
            npc_level: direct_i64(npc, "level")?,
            location: [direct_f64(location, "x")?.to_bits(), direct_f64(location, "y")?.to_bits()],
            npc_type: nested_i64(document, &["npc", "type"])?,
            server_id: nested_i64(document, &["metadata", "server_id"])?,
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
        let Some(time) = nested_i64(document, &["metadata", "mail_time"]) else {
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

        let Some(key) = ReportKey::parse(document) else { return false };
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
    min: Option<i64>,
    max: Option<i64>,
}

impl LootStats {
    fn record(&mut self, quantity: i64) {
        self.seen += 1;
        self.total_quantity += quantity;
        self.min = Some(self.min.map_or(quantity, |value| value.min(quantity)));
        self.max = Some(self.max.map_or(quantity, |value| value.max(quantity)));
    }
}

fn direct_f64(document: &Document, key: &str) -> Option<f64> {
    let value = match document.get(key)? {
        Bson::Double(value) => *value,
        Bson::Int32(value) => f64::from(*value),
        Bson::Int64(value) => *value as f64,
        _ => return None,
    };
    value.is_finite().then_some(value)
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

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn participant(items: &[i32]) -> Document {
        doc! { "player_id": 42_i64, "damage_rate": 100,
        "loot": items.iter().map(|id| doc! {
            "type": 2, "sub_type": id, "value": 1_i64,
        }).collect::<Vec<_>>() }
    }

    fn report(kind: i64, time: i64, items: &[i32]) -> Document {
        doc! { "npc": { "type": kind, "level": 1_i64,
            "location": { "x": 123.0, "y": 456.0 } },
        "metadata": { "server_id": 1234_i64, "mail_time": time },
        "participants": [participant(items)] }
    }

    #[test]
    fn collapsed_bandit_rewards_preserve_optional_pools_and_denominators() {
        let mut stats = AggregateStats::default();
        accumulate_document(&report(70_005, 0, &[9, 75, 47, 7001, 8450]), &mut stats);
        accumulate_document(&report(70_005, 0, &[15, 75, 8450]), &mut stats);
        let document = build_document(70_005, &stats, DateTime::now());
        let slots = document.get_array("slots").expect("slots");
        assert_eq!(slots.len(), 6);
        let special = slots[2].as_document().expect("special pool");
        assert_eq!(special.get_f64("no_item_rate"), Ok(0.5));
        let key = special.get_array("loot").expect("loot")[0].as_document().expect("key");
        assert_eq!(key.get_f64("drop_rate"), Ok(0.5));
        assert_eq!(slots[4].as_document().expect("speedup pool").get_i64("no_item_results"), Ok(2));
        assert_eq!(slots[5].as_document().expect("event pool").get_i64("no_item_results"), Ok(0));
    }

    #[test]
    fn accepts_observed_anniversary_variants() {
        for (kind, items) in
            [(70_005, vec![3, 75, 10004, 4, 4257]), (70_005, vec![3, 65, 10004, 22, 4257])]
        {
            let loot = participant(&items).get_array("loot").expect("loot").clone();
            assert!(parse_slots(kind, &loot).is_some());
        }
    }

    #[test]
    fn malformed_unknown_repeated_and_out_of_order_rewards_are_excluded_atomically() {
        let mut stats = AggregateStats::default();
        for items in [&[9, 75, 123456][..], &[9, 75, 8450, 7001], &[9, 75, 47, 48], &[75, 8450]] {
            accumulate_document(&report(70_005, 0, items), &mut stats);
        }
        let mut missing = participant(&[]);
        missing.remove("loot");
        accumulate_document(
            &doc! { "npc": { "type": 70_005_i64 }, "participants": [missing] },
            &mut stats,
        );
        accumulate_document(&report(70_005, 0, &[]), &mut stats);
        assert_eq!(
            (stats.matched_results, stats.unmatched_results, stats.empty_results),
            (0, 5, 1)
        );
        assert!(stats.slots.iter().all(BTreeMap::is_empty));
        let document = build_document(70_005, &stats, DateTime::now());
        assert!(
            document.get_array("slots").expect("slots").iter().all(|slot| slot
                .as_document()
                .expect("slot")
                .get_f64("no_item_rate")
                == Ok(0.0))
        );
    }

    #[test]
    fn deduplicates_recipient_copies_but_keeps_other_locations_and_later_kills() {
        let first = report(70_005, 999_990, &[9, 75, 8450]);
        let mut copy = report(70_005, 1_000_020, &[9, 75, 8450]);
        copy.get_document_mut("metadata").expect("metadata").insert("mail_receiver", "copy");
        let mut recent = RecentReports::default();
        assert!(!recent.is_duplicate(&first));
        assert!(recent.is_duplicate(&copy));
        let mut elsewhere = copy.clone();
        elsewhere
            .get_document_mut("npc")
            .expect("npc")
            .get_document_mut("location")
            .expect("location")
            .insert("x", 789.0);
        assert!(!recent.is_duplicate(&elsewhere));
        assert!(!recent.is_duplicate(&report(70_005, 4_000_000, &[9, 75, 8450])));
    }
}
