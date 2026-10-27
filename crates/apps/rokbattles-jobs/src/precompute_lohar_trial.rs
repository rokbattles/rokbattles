//! Precompute Lohar's Trial rewards.
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

const NPC_IDS: [i64; 2] = [111, 112];
const AP_COST: i64 = 150;
const REWARD_SLOTS: usize = 5;

const DUPLICATE_WINDOW_MICROS: i64 = 2_000_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LoharTrialPrecomputeStats {
    pub documents_read: usize,
    pub duplicate_reports_skipped: usize,
    pub empty_results_skipped: usize,
    pub unmatched_results: usize,
    pub results_counted: usize,
    pub documents_written: usize,
}

pub async fn precompute_lohar_trial_data(
    reports_store: &ReportsStore,
) -> Result<LoharTrialPrecomputeStats, JobsError> {
    let (documents, mut stats) = compute_lohar_trial_documents(reports_store).await?;
    let output = reports_store.precomputed_lohar_trial_collection();

    for document in documents {
        let kind = document.get_i64("kind").map_err(mongodb::bson::ser::Error::custom)?;
        output.replace_one(doc! { "kind": kind }, document).upsert(true).await?;
        stats.documents_written += 1;
    }

    output.delete_many(doc! { "kind": { "$nin": NPC_IDS.to_vec() } }).await?;

    Ok(stats)
}

pub async fn compute_lohar_trial_documents(
    reports_store: &ReportsStore,
) -> Result<(Vec<Document>, LoharTrialPrecomputeStats), JobsError> {
    let refreshed_at = DateTime::now();
    let (aggregates, stats) = read_observed_reports(
        reports_store.kill_elite_bar_report_collection(),
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
) -> Result<(BTreeMap<i64, AggregateStats>, LoharTrialPrecomputeStats), JobsError> {
    let mut cursor = source
        .aggregate([
            doc! { "$match": {
                "npc.type": { "$in": NPC_IDS.to_vec() },
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
    let mut stats = LoharTrialPrecomputeStats::default();
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
    if slots[0].is_none() || slots[1].is_none() || (kind == 112 && slots[2].is_none()) {
        return None;
    }
    Some(slots)
}

fn slot_for_item(kind: i64, key: LootKey) -> Option<usize> {
    if key.reward_type != 2 {
        return None;
    }
    match (kind, key.sub_type) {
        (111 | 112, 1 | 7 | 26) => Some(0),
        (111, 36 | 37 | 62 | 72 | 82) | (112, 37 | 38 | 64 | 74 | 84) => Some(1),
        (111 | 112, 190 | 10370 | 22098 | 22106 | 22107) | (111, 0) => Some(2),
        (111 | 112, 3 | 9) => Some(3),
        (111, 64 | 74 | 84 | 94) | (112, 65 | 75 | 85 | 95) => Some(4),
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
            "ap_used": usize_to_i64(stats.results).saturating_mul(AP_COST),
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

    fn reward(sub_type: i32, quantity: i64) -> Bson {
        Bson::Document(doc! { "type": 2, "sub_type": sub_type, "value": quantity })
    }

    fn participant(id: i64, loot: Vec<Bson>) -> Document {
        doc! { "player_id": id, "damage_rate": 50.0, "loot": loot }
    }

    fn report(time: i64) -> Document {
        doc! {
            "npc": { "type": 111_i64, "level": 1_i64, "location": { "x": 100.5, "y": 200.5 } },
            "metadata": { "mail_time": time, "server_id": 1234_i64 },
            "participants": [
                participant(1, vec![reward(1, 3), reward(62, 2), reward(0, 0)]),
                participant(2, vec![reward(7, 3), reward(72, 2), reward(10370, 1)]),
            ],
        }
    }

    #[test]
    fn reconstructs_missing_middle_and_final_slots_without_shifting_rewards() {
        let slots = parse_slots(111, &[reward(1, 3), reward(62, 2), reward(74, 1)])
            .expect("Junior's optional third and fourth rolls");
        assert_eq!(
            slots.map(|r| r.map(|r| r.key.sub_type)),
            [Some(1), Some(62), None, None, Some(74)]
        );
        let slots =
            parse_slots(112, &[reward(7, 5), reward(74, 2), reward(10370, 4), reward(95, 2)])
                .expect("Dauntless's optional fourth roll");
        assert_eq!(
            slots.map(|r| r.map(|r| r.key.sub_type)),
            [Some(7), Some(74), Some(10370), None, Some(95)]
        );
    }

    #[test]
    fn treats_explicit_junior_empty_reward_as_a_missing_third_roll() {
        let omitted = parse_slots(111, &[reward(1, 3), reward(62, 2), reward(9, 1)]);
        let explicit = parse_slots(111, &[reward(1, 3), reward(62, 2), reward(0, 0), reward(9, 1)]);
        assert!(explicit.is_some());
        assert_eq!(explicit, omitted);
    }

    #[test]
    fn rejects_unknown_duplicate_out_of_order_and_incomplete_rewards() {
        for (kind, loot) in [
            (111, vec![reward(1, 3), reward(62, 2), reward(99999, 1)]),
            (111, vec![reward(1, 3), reward(62, 2), reward(62, 2)]),
            (111, vec![reward(62, 2), reward(1, 3)]),
            (111, vec![reward(1, 0), reward(62, 2)]),
            (111, vec![reward(1, 3), reward(62, 2), reward(0, 1)]),
            (111, vec![reward(1, 3), reward(10370, 1)]),
            (112, vec![reward(1, 5), reward(74, 2)]),
            (112, vec![reward(1, 5), reward(74, 2), reward(0, 0)]),
            (112, vec![reward(1, 5), reward(62, 2), reward(10370, 1)]),
            (111, vec![reward(1, 3), reward(62, 2), Bson::Null]),
        ] {
            assert!(parse_slots(kind, &loot).is_none(), "kind {kind}: {loot:?}");
        }
    }

    #[test]
    fn counts_results_ap_and_no_item_rates_with_a_shared_matched_denominator() {
        let mut document = report(1);
        document.get_array_mut("participants").expect("participants").extend([
            Bson::Document(participant(3, vec![])),
            Bson::Document(participant(4, vec![reward(99999, 1)])),
        ]);
        let mut aggregate = AggregateStats::default();
        accumulate_document(&document, &mut aggregate);
        let result = build_document(111, &aggregate, DateTime::from_millis(123));
        let totals = result.get_document("totals").expect("totals");
        assert_eq!(totals.get_i64("results"), Ok(3));
        assert_eq!(totals.get_i64("ap_used"), Ok(450));
        assert_eq!(totals.get_i64("empty_results"), Ok(1));
        assert_eq!(totals.get_i64("unmatched_results"), Ok(1));
        assert_eq!(totals.get_i64("matched_results"), Ok(2));
        let third = result.get_array("slots").expect("slots")[2].as_document().expect("slot");
        assert_eq!(third.get_i64("results"), Ok(2));
        assert_eq!(third.get_i64("no_item_results"), Ok(1));
        assert_eq!(third.get_f64("no_item_rate"), Ok(0.5));
        let item = third.get_array("loot").expect("loot")[0].as_document().expect("item");
        assert_eq!(item.get_f64("drop_rate"), Ok(0.5));
        assert_eq!(item.get_i32("sub_type"), Ok(10370));
    }

    #[test]
    fn tracks_quantity_ranges_and_totals_per_slot() {
        let mut document = report(1);
        document.insert(
            "participants",
            vec![
                participant(1, vec![reward(1, 3), reward(62, 2), reward(10370, 1)]),
                participant(2, vec![reward(1, 3), reward(62, 2), reward(10370, 3)]),
            ],
        );
        let mut aggregate = AggregateStats::default();
        accumulate_document(&document, &mut aggregate);
        let reward = aggregate.slots[2].values().next().expect("sculpture");
        assert_eq!(
            (reward.seen, reward.min, reward.max, reward.total_quantity),
            (2, Some(1), Some(3), 4)
        );
    }

    #[test]
    fn emits_five_finite_empty_slots_without_reports() {
        let document = build_document(112, &AggregateStats::default(), DateTime::from_millis(0));
        let slots = document.get_array("slots").expect("slots");
        assert_eq!(slots.len(), 5);
        for slot in slots {
            let slot = slot.as_document().expect("slot");
            assert_eq!(slot.get_f64("no_item_rate"), Ok(0.0));
            assert!(slot.get_array("loot").expect("loot").is_empty());
        }
    }

    #[test]
    fn deduplicates_recipient_copies_with_reordered_participants_and_empty_markers() {
        let mut recent = RecentReports::default();
        assert!(!recent.is_duplicate(&report(10_000_000)));
        let mut copy = report(10_020_000);
        copy.get_array_mut("participants").expect("participants").reverse();
        copy.get_document_mut("metadata").expect("metadata").insert("mail_receiver", "other");
        assert!(recent.is_duplicate(&copy));
        assert!(recent.is_duplicate(&report(12_000_000)));
        assert!(!recent.is_duplicate(&report(12_000_001)));
    }

    #[test]
    fn keeps_distinct_npcs_locations_servers_players_damage_and_rewards() {
        let original = report(1);
        let mut variants = Vec::new();
        for (field, value) in [
            ("type", Bson::Int64(112)),
            ("level", Bson::Int64(2)),
            ("location", Bson::Document(doc! { "x": 101.5, "y": 200.5 })),
        ] {
            let mut variant = original.clone();
            variant.get_document_mut("npc").expect("npc").insert(field, value);
            variants.push(variant);
        }
        let mut server = original.clone();
        server.get_document_mut("metadata").expect("metadata").insert("server_id", 5678_i64);
        variants.push(server);
        for (field, value) in [
            ("player_id", Bson::Int64(3)),
            ("damage_rate", Bson::Double(49.9)),
            ("loot", Bson::Array(vec![reward(7, 3), reward(62, 2)])),
        ] {
            let mut variant = original.clone();
            variant.get_array_mut("participants").expect("participants")[0]
                .as_document_mut()
                .expect("participant")
                .insert(field, value);
            variants.push(variant);
        }
        let mut recent = RecentReports::default();
        assert!(!recent.is_duplicate(&original));
        for variant in variants {
            assert!(!recent.is_duplicate(&variant));
        }
    }

    #[test]
    fn does_not_deduplicate_without_complete_rally_identity() {
        let mut document = report(1);
        document.get_document_mut("npc").expect("npc").remove("location");
        let mut recent = RecentReports::default();
        assert!(!recent.is_duplicate(&document));
        assert!(!recent.is_duplicate(&document));
    }
}
