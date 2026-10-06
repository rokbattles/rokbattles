//! Precompute Karuak Ceremony's five ordered reward slots, excluding empty rewards and mail copies.

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

const BOSS_IDS: [i64; 5] = [30_001, 30_002, 30_003, 30_004, 30_005];
const REWARD_SLOTS: usize = 5;

const DUPLICATE_WINDOW_MICROS: i64 = 2_000_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KaruakCeremonyPrecomputeStats {
    pub documents_read: usize,
    pub duplicate_reports_skipped: usize,
    pub empty_results_skipped: usize,
    pub unmatched_results: usize,
    pub results_counted: usize,
    pub documents_written: usize,
}

pub async fn precompute_karuak_ceremony_data(
    reports_store: &ReportsStore,
) -> Result<KaruakCeremonyPrecomputeStats, JobsError> {
    let (documents, mut stats) = compute_karuak_ceremony_documents(reports_store).await?;
    let output = reports_store.precomputed_karuak_ceremony_collection();

    for document in documents {
        let kind = document.get_i64("kind").map_err(mongodb::bson::ser::Error::custom)?;
        output.replace_one(doc! { "kind": kind }, document).upsert(true).await?;
        stats.documents_written += 1;
    }

    output.delete_many(doc! { "kind": { "$nin": BOSS_IDS.to_vec() } }).await?;

    Ok(stats)
}

/// Compute the scheduled-job output using only database reads, without refreshing data or indexes.
pub async fn compute_karuak_ceremony_documents(
    reports_store: &ReportsStore,
) -> Result<(Vec<Document>, KaruakCeremonyPrecomputeStats), JobsError> {
    let refreshed_at = DateTime::now();
    let (aggregates, stats) = read_observed_reports(
        reports_store.event_member_loot_report_collection(),
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
) -> Result<(BTreeMap<i64, AggregateStats>, KaruakCeremonyPrecomputeStats), JobsError> {
    let mut cursor = source
        .aggregate([
            doc! { "$match": {
                "boss.id": { "$in": BOSS_IDS.to_vec() },
                "metadata.mail_time": { "$gte": cutoff_mail_time },
            } },
            doc! { "$project": {
                "_id": 0,
                "boss.id": 1,
                "metadata.mail_time": 1,
                "metadata.server_id": 1,
                "participants.player_id": 1,
                "participants.loot": 1,
            } },
            doc! { "$sort": { "metadata.mail_time": 1 } },
        ])
        .allow_disk_use(true)
        .await?;

    let mut aggregates: BTreeMap<_, _> =
        BOSS_IDS.into_iter().map(|id| (id, AggregateStats::default())).collect();
    let mut stats = KaruakCeremonyPrecomputeStats::default();
    let mut recent = RecentReports::default();

    while let Some(next) = cursor.next().await {
        let document = next?;
        stats.documents_read += 1;

        let Some(aggregate) =
            nested_i64(&document, &["boss", "id"]).and_then(|id| aggregates.get_mut(&id))
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
    let Ok(participants) = document.get_array("participants") else { return };

    for participant in participants {
        let loot = participant.as_document().and_then(|p| p.get_array("loot").ok());
        if loot.is_some_and(Vec::is_empty) {
            aggregate.empty_results += 1;
            continue;
        }

        let Some(rewards) = loot.and_then(|loot| parse_slots(loot)) else {
            aggregate.unmatched_results += 1;
            continue;
        };

        aggregate.results += 1;
        for (slot, reward) in aggregate.slots.iter_mut().zip(rewards) {
            slot.entry(reward.key).or_default().record(reward.quantity);
        }
    }
}

fn parse_slots(loot: &[Bson]) -> Option<[Reward; REWARD_SLOTS]> {
    if loot.len() != REWARD_SLOTS {
        return None;
    }

    loot.iter().map(Reward::parse).collect::<Option<Vec<_>>>()?.try_into().ok()
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
                        "drop_rate": rate(value.seen, stats.results),
                        "quantity": { "min": value.min.unwrap_or(0), "max": value.max.unwrap_or(0) },
                        "total_quantity": value.total_quantity,
                        "average_quantity": if value.seen == 0 { 0.0 } else {
                            value.total_quantity as f64 / value.seen as f64
                        },
                    }
                })
                .collect::<Vec<_>>();

            doc! {
                "slot": usize_to_i64(index + 1),
                "results": usize_to_i64(stats.results),
                "loot": loot,
            }
        })
        .collect::<Vec<_>>();

    doc! {
        "kind": kind,
        "slots": slots,
        "totals": {
            "results": usize_to_i64(stats.results),
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
    loot: Vec<Reward>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ReportKey {
    boss_id: i64,
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
            boss_id: nested_i64(document, &["boss", "id"])?,
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

    fn participant(id: i64, quantity: i64) -> Document {
        doc! { "player_id": id, "loot": [
            { "type": 2, "sub_type": 5, "value": quantity },
            { "type": 2, "sub_type": 147, "value": 30 },
            { "type": 2, "sub_type": 37, "value": 4 },
            { "type": 2, "sub_type": 48, "value": 1 },
            { "type": 2, "sub_type": 10005, "value": 2 },
        ] }
    }

    fn report(time: i64) -> Document {
        doc! {
            "boss": { "id": 30_005_i64 },
            "metadata": { "mail_time": time, "server_id": 1234_i64 },
            "participants": [participant(1, 1), participant(2, 2)],
        }
    }

    #[test]
    fn preserves_slots_and_quantities_while_excluding_empty_results() {
        let mut stats = AggregateStats::default();

        accumulate_document(
            &doc! { "participants": [
                participant(1, 1), participant(2, 2), { "player_id": 3, "loot": [] },
            ] },
            &mut stats,
        );
        let document = build_document(30_005, &stats, DateTime::from_millis(123));

        assert_eq!(stats.results, 2);
        assert_eq!(stats.empty_results, 1);
        assert_eq!(stats.unmatched_results, 0);

        let slots = document.get_array("slots").expect("slots");
        assert_eq!(slots.len(), 5);

        for (slot, sub_type) in slots.iter().zip([5, 147, 37, 48, 10005]) {
            let slot = slot.as_document().expect("slot");
            let loot = slot.get_array("loot").expect("loot");
            let reward = loot.first().and_then(Bson::as_document).expect("reward");

            assert_eq!(reward.get_i32("sub_type"), Ok(sub_type));
            assert_eq!(reward.get_i64("results"), Ok(2));
            assert_eq!(reward.get_f64("drop_rate"), Ok(1.0));
        }

        let first = stats.slots.first().expect("slot").values().next().expect("reward");
        assert_eq!((first.min, first.max, first.total_quantity), (Some(1), Some(2), 3));
        assert!(!document.contains_key("loot"));
    }

    #[test]
    fn rejects_partial_and_malformed_results_without_counting_any_slots() {
        let mut partial = participant(1, 1);
        partial.get_array_mut("loot").expect("loot").remove(1);

        let mut invalid = participant(2, 1);
        invalid.get_array_mut("loot").expect("loot").push(Bson::Null);

        let zero_quantity = participant(3, 0);

        let mut malformed = participant(4, 1);
        *malformed.get_array_mut("loot").expect("loot").last_mut().expect("reward") = Bson::Null;

        let mut stats = AggregateStats::default();
        accumulate_document(
            &doc! { "participants": [partial, invalid, zero_quantity, malformed, doc! {}] },
            &mut stats,
        );

        assert_eq!(stats.results, 0);
        assert_eq!(stats.unmatched_results, 5);
        assert_eq!(stats.empty_results, 0);
        assert!(stats.slots.iter().all(BTreeMap::is_empty));
    }

    #[test]
    fn emits_five_empty_slots_when_no_rewards_have_been_observed() {
        let doc = build_document(30_001, &AggregateStats::default(), DateTime::from_millis(123));
        let slots = doc.get_array("slots").expect("slots");

        assert_eq!(slots.len(), 5);

        for (index, slot) in slots.iter().enumerate() {
            let slot = slot.as_document().expect("slot");

            assert_eq!(slot.get_i64("slot"), Ok(usize_to_i64(index + 1)));
            assert_eq!(slot.get_i64("results"), Ok(0));
            assert!(slot.get_array("loot").expect("loot").is_empty());
        }
    }

    #[test]
    fn deduplicates_recipient_copies_even_when_participant_order_changes() {
        let original = report(10_000_000);
        let mut copy = report(10_020_000);
        copy.get_array_mut("participants").expect("participants").reverse();
        copy.get_document_mut("metadata").expect("metadata").insert("mail_receiver", "other");

        let mut recent = RecentReports::default();
        assert!(!recent.is_duplicate(&original));
        assert!(recent.is_duplicate(&copy));
        assert!(!recent.is_duplicate(&report(13_000_000)));
    }

    #[test]
    fn keeps_distinct_bosses_servers_players_and_rewards() {
        let mut recent = RecentReports::default();
        assert!(!recent.is_duplicate(&report(10_000_000)));

        let mut other_boss = report(10_001_000);
        other_boss.insert("boss", doc! { "id": 30_004_i64 });
        assert!(!recent.is_duplicate(&other_boss));

        let mut other_server = report(10_002_000);
        other_server.get_document_mut("metadata").expect("metadata").insert("server_id", 5678_i64);
        assert!(!recent.is_duplicate(&other_server));

        let mut other_player = report(10_003_000);
        other_player.insert("participants", vec![participant(3, 1), participant(2, 2)]);
        assert!(!recent.is_duplicate(&other_player));

        let mut other_reward = report(10_004_000);
        other_reward.insert("participants", vec![participant(1, 2), participant(2, 2)]);
        assert!(!recent.is_duplicate(&other_reward));
    }

    #[test]
    fn does_not_deduplicate_without_complete_identity() {
        let mut recent = RecentReports::default();

        let mut missing_server = report(10_000_000);
        missing_server.get_document_mut("metadata").expect("metadata").remove("server_id");
        assert!(!recent.is_duplicate(&missing_server));
        assert!(!recent.is_duplicate(&missing_server));

        let mut no_participants = report(10_000_000);
        no_participants.insert("participants", Vec::<Bson>::new());
        assert!(!recent.is_duplicate(&no_participants));
        assert!(!recent.is_duplicate(&no_participants));
    }
}
