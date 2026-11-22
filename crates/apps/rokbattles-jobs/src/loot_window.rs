use std::{collections::VecDeque, hash::Hash, sync::Arc};

use mongodb::bson::{DateTime, Document};
use rokbattles_bson::nested_i64_exact;
use rustc_hash::FxHashSet;

use crate::loot::{Reward, direct_f64, direct_i64};

const LOOT_WINDOW_DAYS: i64 = 365;
const MILLIS_PER_DAY: i64 = 24 * 60 * 60 * 1_000;

pub(crate) fn loot_cutoff_mail_time(run_at: DateTime) -> i64 {
    run_at
        .timestamp_millis()
        .saturating_sub(LOOT_WINDOW_DAYS * MILLIS_PER_DAY)
        .saturating_mul(1_000)
}

/// Tracks recipient copies relative to the first report, in ascending mail-time order.
pub(crate) struct RecentReports<K, F> {
    parse: F,
    keys: FxHashSet<Arc<K>>,
    arrivals: VecDeque<(i64, Arc<K>)>,
}

impl<K: Eq + Hash, F: Fn(&Document) -> Option<K>> RecentReports<K, F> {
    pub(crate) fn new(parse: F) -> Self {
        Self { parse, keys: FxHashSet::default(), arrivals: VecDeque::new() }
    }

    pub(crate) fn is_duplicate(&mut self, document: &Document) -> bool {
        const DUPLICATE_WINDOW_MICROS: i64 = 2_000_000;
        let Some(time) = nested_i64_exact(document, &["metadata", "mail_time"]) else {
            return false;
        };

        while let Some((_, key)) = self
            .arrivals
            .pop_front_if(|(first, _)| time.saturating_sub(*first) > DUPLICATE_WINDOW_MICROS)
        {
            self.keys.remove(&key);
        }

        let Some(key) = (self.parse)(document) else { return false };
        let key = Arc::new(key);
        if !self.keys.insert(Arc::clone(&key)) {
            return true;
        }
        self.arrivals.push_back((time, key));
        false
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ParticipantOutcome {
    player_id: i64,
    damage: u64,
    loot: Vec<Reward>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct NpcReportKey {
    npc_type: i64,
    npc_level: i64,
    location: [u64; 2],
    server_id: i64,
    participants: Vec<ParticipantOutcome>,
}

impl NpcReportKey {
    pub(crate) fn parse(document: &Document) -> Option<Self> {
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
                        .map(Reward::with_empty_slot)
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
            npc_type: direct_i64(npc, "type")?,
            server_id: nested_i64_exact(document, &["metadata", "server_id"])?,
            participants,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cutoff_preserves_utc_time_and_converts_to_microseconds() {
        let run_at =
            DateTime::parse_rfc3339_str("2026-10-05T12:34:56.789Z").expect("run timestamp");
        let expected = DateTime::parse_rfc3339_str("2025-10-05T12:34:56.789Z")
            .expect("cutoff timestamp")
            .timestamp_millis()
            * 1_000;

        assert_eq!(loot_cutoff_mail_time(run_at), expected);
    }

    #[test]
    fn cutoff_uses_365_days_across_a_leap_year() {
        let run_at = DateTime::parse_rfc3339_str("2024-03-01T08:00:00Z").expect("run timestamp");
        let expected = DateTime::parse_rfc3339_str("2023-03-02T08:00:00Z")
            .expect("cutoff timestamp")
            .timestamp_millis()
            * 1_000;

        assert_eq!(loot_cutoff_mail_time(run_at), expected);
    }
}
