use mongodb::bson::{Bson, Document, doc};
use rokbattles_bson::{nested_i64, nested_str};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::BATTLE_LIFETIME_MS;
use crate::routes::reports::battle::{
    list_mapper::{build_report_dedupe_key, map_battle_list_document},
    types::ReportListParticipant,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct March {
    pub governor: i64,
    pub server: i64,
    pub tracking: String,
    pub mail: String,
}

impl March {
    pub fn from_report(report: &Document) -> Option<Self> {
        let tracking = nested_str(report, &["sender", "tracking_key"]).unwrap_or_default();

        Some(Self {
            governor: nested_i64(report, &["sender", "player_id"])?,
            server: nested_i64(report, &["metadata", "server_id"]).unwrap_or(0),
            tracking: tracking.to_owned(),
            mail: if tracking.is_empty() {
                nested_str(report, &["metadata", "mail_id"])?.to_owned()
            } else {
                String::new()
            },
        })
    }

    pub fn filter(&self) -> Document {
        if self.tracking.is_empty() {
            doc! { "metadata.mail_id": &self.mail }
        } else {
            doc! {
                "sender.player_id": self.governor,
                "metadata.server_id": self.server,
                "sender.tracking_key": &self.tracking
            }
        }
    }

    fn id(&self) -> String {
        let identity = format!("{}|{}|{}|{}", self.governor, self.server, self.tracking, self.mail);

        format!("{:x}", Sha256::digest(identity.as_bytes()))
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Outcome {
    Victory,
    Defeat,
    Battle,
    Unknown,
}

fn outcome(remaining_units: impl Iterator<Item = (Option<i64>, Option<i64>)>) -> Outcome {
    let mut known = false;
    let mut victory = false;

    for (sender_remaining, opponent_remaining) in remaining_units {
        match (sender_remaining, opponent_remaining) {
            (Some(0), _) => return Outcome::Defeat,
            (Some(sender), Some(0)) if sender > 0 => {
                known = true;
                victory = true;
            }
            (Some(sender), Some(opponent)) if sender > 0 && opponent > 0 => known = true,
            _ => {}
        }
    }

    if victory {
        Outcome::Victory
    } else if known {
        Outcome::Battle
    } else {
        Outcome::Unknown
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BattleItem {
    pub id: String,
    pub commanders: ReportListParticipant,
    pub outcome: Outcome,
    pub kill_count: i64,
    pub kill_points: i64,
    pub conceded_kill_points: i64,
    pub trade_percent: Option<i64>,
    pub report_count: usize,
    pub updated_at: i64,
    pub expires_at: i64,
    #[serde(skip)]
    pub latest_mail_id: String,
}

pub(super) fn aggregate(reports: Vec<Document>) -> Vec<BattleItem> {
    let mut groups: FxHashMap<March, BattleItem> = FxHashMap::default();
    let mut seen = FxHashSet::default();

    for mut report in reports {
        let Some(march) = March::from_report(&report) else { continue };

        let dedupe = build_report_dedupe_key(&report)
            .or_else(|| nested_str(&report, &["metadata", "mail_id"]).map(str::to_owned));

        if !seen.insert((march.clone(), dedupe)) {
            continue;
        }

        let Ok(opponents) = report.get_array_mut("opponents") else { continue };
        let original_opponent_count = opponents.len();

        opponents.retain(|value| {
            value.as_document().is_some_and(|opponent| {
                nested_i64(opponent, &["player_id"]).is_some_and(|id| id > 2)
            })
        });

        if opponents.is_empty() {
            continue;
        }

        let result = outcome(opponents.iter().filter_map(Bson::as_document).map(|opponent| {
            (
                nested_i64(opponent, &["battle_results", "sender", "remaining"]),
                nested_i64(opponent, &["battle_results", "opponent", "remaining"]),
            )
        }));

        if opponents.len() != original_opponent_count {
            report.remove("summary");
        }

        let Some(row) = map_battle_list_document(&report) else { continue };
        let updated_at = row.mail_time / 1000;
        let item = row.item;

        let group = groups.entry(march.clone()).or_insert_with(|| BattleItem {
            id: march.id(),
            commanders: item.sender,
            outcome: result,
            kill_count: 0,
            kill_points: 0,
            conceded_kill_points: 0,
            trade_percent: None,
            report_count: 0,
            updated_at,
            expires_at: updated_at + BATTLE_LIFETIME_MS,
            latest_mail_id: item.mail_id.clone(),
        });

        group.kill_count = group.kill_count.saturating_add(item.kill_count);
        group.kill_points = group.kill_points.saturating_add(item.summary.sender.kill_points);
        group.conceded_kill_points =
            group.conceded_kill_points.saturating_add(item.summary.opponent.kill_points);
        group.report_count += 1;

        if updated_at > group.updated_at
            || (updated_at == group.updated_at && item.mail_id > group.latest_mail_id)
        {
            group.updated_at = updated_at;
            group.expires_at = updated_at + BATTLE_LIFETIME_MS;
            group.latest_mail_id = item.mail_id;
            group.outcome = result;
        }
    }

    let mut items: Vec<_> = groups.into_values().collect();

    for item in &mut items {
        item.trade_percent = if item.conceded_kill_points > 0 {
            Some((item.kill_points as f64 / item.conceded_kill_points as f64 * 100.0).round() as i64)
        } else if item.kill_points == 0 {
            Some(100)
        } else {
            None
        };
    }

    items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then_with(|| a.id.cmp(&b.id)));

    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(mail: &str, tracking: &str, time: i64, sender_remaining: i64) -> Document {
        doc! {
            "metadata": {"mail_id": mail, "mail_time": time * 1000, "server_id": 1},
            "sender": {
                "player_id": 10,
                "tracking_key": tracking,
                "commanders": {"primary": {"id": 575}, "secondary": {"id": 579}}
            },
            "opponents": [{
                "player_id": 20,
                "battle_results": {
                    "sender": {"kill_points": 200, "remaining": sender_remaining},
                    "opponent": {"kill_points": 100, "dead": 5, "severely_wounded": 10, "remaining": 0}
                }
            }]
        }
    }

    #[test]
    fn merges_march_dedupes_mail_and_uses_latest_result() {
        let a = report("a", "march", 1000, 100);
        let b = report("b", "march", 2000, 0);
        let rows = aggregate(vec![b, a.clone(), a]);

        assert_eq!(rows.len(), 1);
        assert_eq!(
            (rows[0].kill_count, rows[0].trade_percent, rows[0].report_count),
            (30, Some(200), 2)
        );
        assert_eq!((rows[0].outcome, rows[0].expires_at), (Outcome::Defeat, 302_000));
    }

    #[test]
    fn missing_tracking_keys_stay_separate_and_each_march_expires() {
        let rows = aggregate(vec![report("a", "", 1000, 2), report("b", "", 2000, 0)]);

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].expires_at, 302_000);
        assert_eq!(rows[1].expires_at, 301_000);
    }

    #[test]
    fn excludes_npcs_even_in_mixed_reports() {
        let mut mixed = report("a", "march", 1000, 2);

        for id in [-2, 0, 1, 2] {
            mixed.get_array_mut("opponents").unwrap().push(Bson::Document(doc! {
                "player_id": id,
                "battle_results": {"sender": {"remaining": 0}, "opponent": {"dead": 999}}
            }));
        }

        mixed.insert("summary", doc! {"opponent": {"dead": 4001, "severely_wounded": 10}});

        let rows = aggregate(vec![mixed]);

        assert_eq!(rows[0].kill_count, 15);
        assert_eq!(rows[0].outcome, Outcome::Victory);
    }

    #[test]
    fn requires_an_opponent_with_pid_greater_than_two() {
        for id in [-2, 0, 1, 2, 3] {
            let mut mail = report("a", "march", 1000, 2);
            mail.get_array_mut("opponents").unwrap()[0]
                .as_document_mut()
                .unwrap()
                .insert("player_id", id);

            assert_eq!(aggregate(vec![mail]).len(), usize::from(id > 2));
        }
    }

    #[test]
    fn trade_is_ratio_of_totals_and_handles_zero_losses() {
        let mut b = report("b", "march", 2000, 2);
        b.insert(
            "summary",
            doc! {"sender": {"kill_points": 100}, "opponent": {"kill_points": 900}},
        );

        assert_eq!(aggregate(vec![report("a", "march", 1000, 2), b])[0].trade_percent, Some(30));

        let mut c = report("c", "march", 1000, 2);
        c.insert("summary", doc! {"opponent": {"kill_points": 0}});

        assert_eq!(aggregate(vec![c])[0].trade_percent, None);
    }

    #[test]
    fn remaining_units_determine_the_result_without_treating_missing_counts_as_zero() {
        assert_eq!(outcome([(Some(100), Some(0))].into_iter()), Outcome::Victory);
        assert_eq!(outcome([(Some(0), Some(100))].into_iter()), Outcome::Defeat);
        assert_eq!(outcome([(Some(0), Some(0))].into_iter()), Outcome::Defeat);
        assert_eq!(outcome([(Some(100), Some(50))].into_iter()), Outcome::Battle);
        assert_eq!(outcome([(None, Some(0))].into_iter()), Outcome::Unknown);
        assert_eq!(outcome([(Some(100), None)].into_iter()), Outcome::Unknown);
        assert_eq!(outcome([(None, None)].into_iter()), Outcome::Unknown);
        assert_eq!(
            outcome([(Some(100), Some(0)), (Some(0), Some(100))].into_iter()),
            Outcome::Defeat
        );
    }
}
