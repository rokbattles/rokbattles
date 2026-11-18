use mongodb::bson::Document;
use rustc_hash::FxHashMap;

use super::{
    aggregate::{
        PairingEntry, build_loadout_key, build_loadout_snapshot, for_each_pairing_entry,
        mail_is_in_range,
    },
    types::{
        LoadoutSnapshot, PairingDetailLoadout, PairingDetailResponse, PairingMatchup, PairingStats,
        PairingsRange,
    },
};
use crate::routes::governor::date_range::GovernorDateRange;

#[derive(Default)]
struct StatsAccumulator {
    battles: i64,
    kp_gain: i64,
    kp_loss: i64,
}

impl StatsAccumulator {
    fn push(&mut self, entry: &PairingEntry) {
        self.battles += 1;
        self.kp_gain += entry.delta.kill_score;
        self.kp_loss += entry.delta.enemy_kill_score;
    }

    fn finish(self) -> PairingStats {
        PairingStats {
            battles: self.battles,
            kp_gain: self.kp_gain,
            kp_loss: self.kp_loss,
            net_kp: self.kp_gain - self.kp_loss,
            trade_percent: if self.kp_loss > 0 {
                Some(self.kp_gain as f64 / self.kp_loss as f64 * 100.0)
            } else {
                None
            },
        }
    }
}

pub(super) fn aggregate_detail(
    mails: &[Document],
    range: &GovernorDateRange,
    primary: i64,
    secondary: i64,
) -> PairingDetailResponse {
    let mut total = StatsAccumulator::default();
    let mut buckets: FxHashMap<String, (LoadoutSnapshot, StatsAccumulator)> = FxHashMap::default();

    for mail in mails {
        if !mail_is_in_range(mail, range) {
            continue;
        }

        let snapshot = build_loadout_snapshot(mail);
        let key = build_loadout_key(&snapshot);

        for_each_pairing_entry(mail, |entry| {
            if entry.self_primary_commander_id != primary
                || entry.self_secondary_commander_id != secondary
            {
                return;
            }

            total.push(&entry);
            buckets
                .entry(key.clone())
                .or_insert_with(|| (snapshot.clone(), StatsAccumulator::default()))
                .1
                .push(&entry);
        });
    }

    let mut loadouts: Vec<_> = buckets
        .into_iter()
        .map(|(key, (loadout, stats))| PairingDetailLoadout { key, stats: stats.finish(), loadout })
        .collect();
    loadouts.sort_by(|a, b| b.stats.battles.cmp(&a.stats.battles).then_with(|| a.key.cmp(&b.key)));

    PairingDetailResponse {
        range: PairingsRange { start: range.start.clone(), end: range.end.clone() },
        pairing: (total.battles > 0).then(|| total.finish()),
        loadouts,
    }
}

pub(super) fn aggregate_matchups(
    mails: &[Document],
    range: &GovernorDateRange,
    primary: i64,
    secondary: i64,
    loadout_key: Option<&str>,
) -> Vec<PairingMatchup> {
    let mut buckets: FxHashMap<(i64, i64), StatsAccumulator> = FxHashMap::default();

    for mail in mails {
        if !mail_is_in_range(mail, range)
            || loadout_key
                .is_some_and(|key| build_loadout_key(&build_loadout_snapshot(mail)) != key)
        {
            continue;
        }

        for_each_pairing_entry(mail, |entry| {
            if entry.self_primary_commander_id == primary
                && entry.self_secondary_commander_id == secondary
            {
                buckets
                    .entry((entry.enemy_primary_commander_id, entry.enemy_secondary_commander_id))
                    .or_default()
                    .push(&entry);
            }
        });
    }

    let mut items: Vec<_> = buckets
        .into_iter()
        .filter(|(_, stats)| stats.battles >= 100)
        .map(|((primary_commander_id, secondary_commander_id), stats)| PairingMatchup {
            primary_commander_id,
            secondary_commander_id,
            stats: stats.finish(),
        })
        .collect();
    items.sort_by(|a, b| {
        b.stats
            .battles
            .cmp(&a.stats.battles)
            .then_with(|| a.primary_commander_id.cmp(&b.primary_commander_id))
            .then_with(|| a.secondary_commander_id.cmp(&b.secondary_commander_id))
    });

    items
}

#[cfg(test)]
mod tests {
    use mongodb::bson::doc;

    use super::*;

    fn range() -> GovernorDateRange {
        GovernorDateRange {
            start: "2025-01-01".to_string(),
            end: "2025-01-01".to_string(),
            start_millis: 1_735_689_600_000,
            end_millis: 1_735_776_000_000,
        }
    }

    fn mail(equipment: &str, enemy: i64, gained: i64, lost: i64) -> Document {
        doc! {
            "metadata": { "mail_time": range().start_millis },
            "sender": { "commanders": {
                "primary": { "id": 100_i64, "equipment": equipment,
                    "armaments": [{ "affix": "101;202", "buffs": "1000_0.025" }] },
                "secondary": { "id": 200_i64 },
            } },
            "opponents": [{
                "player_id": 999_i64,
                "commanders": { "primary": { "id": enemy }, "secondary": { "id": 0_i64 } },
                "battle_results": {
                    "sender": { "kill_points": gained },
                    "opponent": { "kill_points": lost },
                },
            }],
        }
    }

    #[test]
    fn detail_combines_all_loadouts_and_preserves_each_build() {
        let response = aggregate_detail(
            &[
                mail("{1:500_2:25}", 300, 100, 10),
                mail("{1:500_2:25}", 300, 100, 190),
                mail("{1:600_2:25}", 400, 50, 100),
            ],
            &range(),
            100,
            200,
        );
        assert_eq!(
            response.pairing,
            Some(PairingStats {
                battles: 3,
                kp_gain: 250,
                kp_loss: 300,
                net_kp: -50,
                trade_percent: Some(250.0 / 300.0 * 100.0),
            })
        );
        assert_eq!(response.loadouts.len(), 2);
        let most_used = &response.loadouts[0];
        assert_eq!(
            most_used.stats,
            PairingStats {
                battles: 2,
                kp_gain: 200,
                kp_loss: 200,
                net_kp: 0,
                trade_percent: Some(100.0),
            }
        );
        assert_eq!(most_used.loadout.equipment[0].id, 500);
        assert_eq!(most_used.loadout.inscriptions, [101, 202]);
        assert_eq!(most_used.loadout.armaments[0].value, 0.025);
    }

    #[test]
    fn detail_returns_no_data_for_another_pairing() {
        let response = aggregate_detail(&[mail("", 300, 100, 50)], &range(), 100, 201);
        assert!(response.pairing.is_none() && response.loadouts.is_empty());
    }

    #[test]
    fn detail_counts_each_human_opponent_and_ignores_npcs() {
        let mut report = mail("", 300, 100, 50);
        let opponents = report.get_array_mut("opponents").expect("opponents");
        let mut npc = opponents[0].as_document().expect("first opponent").clone();
        npc.insert("player_id", 0_i64);
        opponents.extend([opponents[0].clone(), npc.into()]);
        let response = aggregate_detail(&[report], &range(), 100, 200);
        let stats = response.pairing.expect("human battles");
        assert_eq!((stats.battles, stats.kp_gain, stats.kp_loss), (2, 200, 100));
        assert_eq!(response.loadouts[0].stats, stats);
    }

    #[test]
    fn detail_excludes_reports_at_the_end_boundary() {
        let mut boundary = mail("", 300, 999, 999);
        boundary
            .get_document_mut("metadata")
            .expect("metadata")
            .insert("mail_time", range().end_millis);
        let response = aggregate_detail(&[mail("", 300, 100, 50), boundary], &range(), 100, 200);
        assert_eq!(response.pairing.expect("one in-range battle").battles, 1);
    }

    #[test]
    fn matchups_default_to_battles_descending_and_require_one_hundred_battles() {
        let mails = std::iter::repeat_n(mail("", 300, 1_000, 50), 100)
            .chain(std::iter::repeat_n(mail("", 400, 10, 50), 200))
            .chain(std::iter::repeat_n(mail("", 500, 10_000, 50), 99))
            .collect::<Vec<_>>();
        let items = aggregate_matchups(&mails, &range(), 100, 200, None);
        assert_eq!(
            items
                .iter()
                .map(|item| (item.primary_commander_id, item.stats.battles))
                .collect::<Vec<_>>(),
            [(400, 200), (300, 100)]
        );
    }

    #[test]
    fn selected_loadout_matchups_do_not_include_another_build() {
        let mails = std::iter::repeat_n(mail("{1:500}", 300, 100, 50), 100)
            .chain(std::iter::repeat_n(mail("{1:600}", 300, 10, 50), 200))
            .collect::<Vec<_>>();
        let detail = aggregate_detail(&mails, &range(), 100, 200);
        let selected = detail
            .loadouts
            .iter()
            .find(|item| item.loadout.equipment[0].id == 500)
            .expect("selected build");
        let items = aggregate_matchups(&mails, &range(), 100, 200, Some(&selected.key));
        assert_eq!(items[0].stats, selected.stats);
    }

    #[test]
    fn all_loadouts_matchups_include_every_build() {
        let mails = std::iter::repeat_n(mail("{1:500}", 300, 100, 50), 100)
            .chain(std::iter::repeat_n(mail("{1:600}", 300, 10, 50), 200))
            .collect::<Vec<_>>();
        let detail = aggregate_detail(&mails, &range(), 100, 200);
        let items = aggregate_matchups(&mails, &range(), 100, 200, None);
        assert_eq!(Some(items[0].stats.clone()), detail.pairing);
    }

    #[test]
    fn unknown_loadout_matchups_are_empty() {
        let mails = vec![mail("", 300, 100, 50); 100];
        assert!(aggregate_matchups(&mails, &range(), 100, 200, Some("unknown")).is_empty());
    }

    #[test]
    fn lossless_trade_serializes_as_null_instead_of_a_misleading_percentage() {
        let response = aggregate_detail(&[mail("", 300, 100, 0)], &range(), 100, 200);
        let value = serde_json::to_value(response).expect("valid JSON");
        assert_eq!(value["pairing"]["tradePercent"], serde_json::Value::Null);
    }
}
