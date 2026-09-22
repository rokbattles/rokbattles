use std::cmp::Ordering;

use mongodb::bson::{Bson, Document};
use rokbattles_bson::{nested_bool, nested_document, nested_i64, nested_str};
use sha2::{Digest, Sha256};

use super::types::{
    ReportListItem, ReportListParticipant, ReportRowWithCursor, ReportSummary, ReportSummaryEntry,
    ReportTimeline, TimelineSample,
};

pub(crate) fn build_battle_list_projection() -> Document {
    let mut projection = Document::new();

    for field in [
        "metadata.mail_id",
        "metadata.mail_time",
        "metadata.server_id",
        "metadata.kvk",
        "metadata.mail_role",
        "sender.supreme_strife.battle_id",
        "sender.supreme_strife.team_id",
        "timeline.start_timestamp",
        "timeline.end_timestamp",
        "timeline.sampling.tick",
        "timeline.sampling.count",
        "sender.player_id",
        "sender.tracking_key",
        "sender.kingdom_id",
        "sender.session",
        "sender.rally",
        "sender.alliance_building_id",
        "sender.structure_id",
        "sender.commanders.primary.id",
        "sender.commanders.primary.awakened",
        "sender.commanders.secondary.id",
        "sender.commanders.secondary.awakened",
        "summary.sender.kill_points",
        "summary.sender.dead",
        "summary.sender.severely_wounded",
        "summary.sender.slightly_wounded",
        "summary.sender.remaining",
        "summary.sender.troop_units",
        "summary.opponent.kill_points",
        "summary.opponent.dead",
        "summary.opponent.severely_wounded",
        "summary.opponent.slightly_wounded",
        "summary.opponent.remaining",
        "summary.opponent.troop_units",
        "opponents.player_id",
        "opponents.tracking_key",
        "opponents.rally",
        "opponents.attack.id",
        "opponents.start_tick",
        "opponents.alliance_building_id",
        "opponents.structure_id",
        "opponents.commanders.primary.id",
        "opponents.commanders.primary.awakened",
        "opponents.commanders.secondary.id",
        "opponents.commanders.secondary.awakened",
        "opponents.battle_results.sender.kill_points",
        "opponents.battle_results.sender.dead",
        "opponents.battle_results.sender.severely_wounded",
        "opponents.battle_results.sender.slightly_wounded",
        "opponents.battle_results.sender.remaining",
        "opponents.battle_results.sender.troop_units",
        "opponents.battle_results.opponent.kill_points",
        "opponents.battle_results.opponent.dead",
        "opponents.battle_results.opponent.severely_wounded",
        "opponents.battle_results.opponent.slightly_wounded",
        "opponents.battle_results.opponent.remaining",
        "opponents.battle_results.opponent.troop_units",
    ] {
        projection.insert(field, 1);
    }

    projection
}

pub(super) fn build_report_dedupe_key(document: &Document) -> Option<String> {
    if !is_shared_combat_report(document) {
        return build_field_report_dedupe_key(document).or_else(|| {
            nested_str(document, &["metadata", "mail_id"]).map(|mail_id| format!("mail:{mail_id}"))
        });
    }

    let sender_player_id = nested_i64(document, &["sender", "player_id"]).unwrap_or(0);
    let server_id = nested_i64(document, &["metadata", "server_id"]).unwrap_or(0);
    let start_timestamp = nested_i64(document, &["timeline", "start_timestamp"]).unwrap_or(0);
    let end_timestamp = nested_i64(document, &["timeline", "end_timestamp"]).unwrap_or(0);

    let mut attack_ids = extract_opponents(document)
        .into_iter()
        .filter(|opponent| is_valid_opponent(opponent))
        .filter_map(extract_attack_id)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    attack_ids.sort_unstable();
    attack_ids.dedup();

    if attack_ids.is_empty() {
        return nested_str(document, &["metadata", "mail_id"])
            .map(|mail_id| format!("mail:{mail_id}"));
    }

    Some(format!(
        "attacks:{}|sender:{sender_player_id}|server:{server_id}|start:{start_timestamp}|end:{end_timestamp}",
        attack_ids.join(",")
    ))
}

fn build_field_report_dedupe_key(document: &Document) -> Option<String> {
    let sender = field_participant_identity(nested_document(document, &["sender"])?)?;
    let server = nested_i64(document, &["metadata", "server_id"]).filter(|id| *id > 0)?;
    let start = nested_i64(document, &["timeline", "start_timestamp"])?;
    let end = nested_i64(document, &["timeline", "end_timestamp"])?;
    let opponents = document.get_array("opponents").ok()?;
    if opponents.is_empty() {
        return None;
    }

    let mut entries = Vec::with_capacity(opponents.len());
    for opponent in opponents {
        let opponent = opponent.as_document()?;
        let attack_id = extract_attack_id(opponent).filter(|id| !id.is_empty())?;
        let identity = field_participant_identity(opponent)?;
        entries.push((attack_id, identity));
    }
    // Different marches can share every attack ID and the same battle window.
    // Keep march identity and repeated entries; only ignore entry order.
    entries.sort_unstable();
    let identity = serde_json::to_vec(&(server, sender, start, end, entries)).ok()?;
    Some(format!("field:{:x}", Sha256::digest(identity)))
}

fn field_participant_identity(participant: &Document) -> Option<(i64, &str)> {
    Some((
        nested_i64(participant, &["player_id"]).filter(|id| *id > 0)?,
        nested_str(participant, &["tracking_key"]).filter(|key| !key.is_empty())?,
    ))
}

fn is_shared_combat_report(document: &Document) -> bool {
    if nested_bool(document, &["sender", "rally"]) == Some(true)
        || nested_document(document, &["sender"]).is_some_and(has_garrison_field)
    {
        return true;
    }

    extract_opponents(document).into_iter().any(|opponent| {
        is_valid_opponent(opponent)
            && (nested_bool(opponent, &["rally"]) == Some(true) || has_garrison_field(opponent))
    })
}

fn has_garrison_field(participant: &Document) -> bool {
    has_non_null_field(participant, "alliance_building_id")
        || has_non_null_field(participant, "structure_id")
}

pub(crate) fn map_battle_list_document(document: &Document) -> Option<ReportRowWithCursor> {
    let mail_id = nested_str(document, &["metadata", "mail_id"])?.to_string();
    let mail_time = nested_i64(document, &["metadata", "mail_time"])?;

    let opponents = extract_opponents(document);
    let valid_opponents = get_valid_sorted_opponents(&opponents);
    let preferred_opponent = select_preferred_opponent(&valid_opponents)?;

    let sender_summary = resolve_summary_entry(document, "sender", &opponents);
    let opponent_summary = resolve_summary_entry(document, "opponent", &opponents);

    let time_start = nested_i64(document, &["timeline", "start_timestamp"]).unwrap_or(mail_time);
    let time_end = nested_i64(document, &["timeline", "end_timestamp"]).unwrap_or(time_start);

    let battles = i64::try_from(valid_opponents.len()).unwrap_or(i64::MAX);
    let kill_count = opponent_summary.dead + opponent_summary.severely_wounded;
    let trade_percent =
        compute_trade_percent(sender_summary.kill_points, opponent_summary.kill_points);

    let item = ReportListItem {
        mail_id,
        kvk_mapcode: "Unknown".into(),
        kvk_banner: None,
        time_start,
        time_end,
        sender: ReportListParticipant {
            primary_commander_id: nested_i64(document, &["sender", "commanders", "primary", "id"])
                .unwrap_or(0),
            primary_commander_awakened: nested_bool(
                document,
                &["sender", "commanders", "primary", "awakened"],
            ),
            secondary_commander_id: nested_i64(
                document,
                &["sender", "commanders", "secondary", "id"],
            )
            .unwrap_or(0),
            secondary_commander_awakened: nested_bool(
                document,
                &["sender", "commanders", "secondary", "awakened"],
            ),
        },
        opponent: ReportListParticipant {
            primary_commander_id: nested_i64(preferred_opponent, &["commanders", "primary", "id"])
                .unwrap_or(0),
            primary_commander_awakened: nested_bool(
                preferred_opponent,
                &["commanders", "primary", "awakened"],
            ),
            secondary_commander_id: nested_i64(
                preferred_opponent,
                &["commanders", "secondary", "id"],
            )
            .unwrap_or(0),
            secondary_commander_awakened: nested_bool(
                preferred_opponent,
                &["commanders", "secondary", "awakened"],
            ),
        },
        battles,
        kill_count,
        trade_percent,
        summary: ReportSummary { sender: sender_summary, opponent: opponent_summary },
        timeline: ReportTimeline {
            start_timestamp: nested_i64(document, &["timeline", "start_timestamp"])
                .unwrap_or(time_start),
            end_timestamp: nested_i64(document, &["timeline", "end_timestamp"]).unwrap_or(time_end),
            sampling: extract_timeline_samples(document),
        },
    };

    Some(ReportRowWithCursor {
        mail_time,
        map_context: super::map_context::BattleMapContext::from_document(document, time_start),
        item,
    })
}

pub(crate) fn compute_trade_percent(sender_kill_points: i64, opponent_kill_points: i64) -> i64 {
    if sender_kill_points == opponent_kill_points {
        return 100;
    }

    if opponent_kill_points <= 0 {
        return 0;
    }

    ((sender_kill_points as f64 / opponent_kill_points as f64) * 100.0).round() as i64
}

fn extract_opponents(document: &Document) -> Vec<&Document> {
    document
        .get_array("opponents")
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Bson::as_document)
        .collect()
}

fn extract_attack_id(opponent: &Document) -> Option<String> {
    let attack = opponent.get_document("attack").ok()?;
    let raw_id = attack.get("id")?;
    Some(match raw_id {
        Bson::String(value) => value.clone(),
        Bson::Int32(value) => value.to_string(),
        Bson::Int64(value) => value.to_string(),
        Bson::Double(value) => value.to_string(),
        _ => String::new(),
    })
}

fn get_valid_sorted_opponents<'a>(opponents: &'a [&Document]) -> Vec<&'a Document> {
    let mut valid = opponents
        .iter()
        .copied()
        .filter(|opponent| is_valid_opponent(opponent))
        .collect::<Vec<_>>();

    valid.sort_by(|a, b| {
        let a_tick = nested_i64(a, &["start_tick"]).unwrap_or(0);
        let b_tick = nested_i64(b, &["start_tick"]).unwrap_or(0);
        match a_tick.cmp(&b_tick) {
            Ordering::Equal => {
                let a_player = nested_i64(a, &["player_id"]).unwrap_or(0);
                let b_player = nested_i64(b, &["player_id"]).unwrap_or(0);
                a_player.cmp(&b_player)
            }
            order => order,
        }
    });

    valid
}

fn is_valid_opponent(opponent: &Document) -> bool {
    let player_id = nested_i64(opponent, &["player_id"]).unwrap_or(0);
    player_id > 0
}

fn select_preferred_opponent<'a>(opponents: &'a [&Document]) -> Option<&'a Document> {
    for opponent in opponents {
        if has_non_null_field(opponent, "alliance_building_id")
            || has_non_null_field(opponent, "structure_id")
        {
            return Some(opponent);
        }
    }

    opponents.first().copied()
}

fn has_non_null_field(document: &Document, key: &str) -> bool {
    matches!(document.get(key), Some(value) if value.as_null().is_none())
}

fn resolve_summary_entry(
    document: &Document,
    side: &str,
    opponents: &[&Document],
) -> ReportSummaryEntry {
    let fallback = build_fallback_summary(side, opponents);
    let source = nested_document(document, &["summary", side]);

    ReportSummaryEntry {
        troop_units: source
            .and_then(|value| nested_i64(value, &["troop_units"]))
            .unwrap_or(fallback.troop_units),
        dead: source.and_then(|value| nested_i64(value, &["dead"])).unwrap_or(fallback.dead),
        severely_wounded: source
            .and_then(|value| nested_i64(value, &["severely_wounded"]))
            .unwrap_or(fallback.severely_wounded),
        slightly_wounded: source
            .and_then(|value| nested_i64(value, &["slightly_wounded"]))
            .unwrap_or(fallback.slightly_wounded),
        remaining: source
            .and_then(|value| nested_i64(value, &["remaining"]))
            .unwrap_or(fallback.remaining),
        kill_points: source
            .and_then(|value| nested_i64(value, &["kill_points"]))
            .unwrap_or(fallback.kill_points),
    }
}

fn build_fallback_summary(side: &str, opponents: &[&Document]) -> ReportSummaryEntry {
    let mut summary = ReportSummaryEntry {
        troop_units: 0,
        dead: 0,
        severely_wounded: 0,
        slightly_wounded: 0,
        remaining: 0,
        kill_points: 0,
    };

    for opponent in opponents.iter().filter(|opponent| is_valid_opponent(opponent)) {
        summary.troop_units +=
            nested_i64(opponent, &["battle_results", side, "troop_units"]).unwrap_or(0);
        summary.dead += nested_i64(opponent, &["battle_results", side, "dead"]).unwrap_or(0);
        summary.severely_wounded +=
            nested_i64(opponent, &["battle_results", side, "severely_wounded"]).unwrap_or(0);
        summary.slightly_wounded +=
            nested_i64(opponent, &["battle_results", side, "slightly_wounded"]).unwrap_or(0);
        summary.remaining +=
            nested_i64(opponent, &["battle_results", side, "remaining"]).unwrap_or(0);
        summary.kill_points +=
            nested_i64(opponent, &["battle_results", side, "kill_points"]).unwrap_or(0);
    }

    summary
}

fn extract_timeline_samples(document: &Document) -> Vec<TimelineSample> {
    document
        .get_document("timeline")
        .ok()
        .and_then(|timeline| timeline.get_array("sampling").ok())
        .into_iter()
        .flatten()
        .filter_map(Bson::as_document)
        .filter_map(|sample| {
            let tick = nested_i64(sample, &["tick"])?;
            let count = nested_i64(sample, &["count"])?;
            if count < 0 {
                return None;
            }

            Some(TimelineSample { tick, count })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use mongodb::bson::doc;

    use super::*;

    #[test]
    fn dedupe_key_sorts_and_deduplicates_attack_ids() {
        let document = doc! {
            "metadata": { "mail_id": "mail-1", "server_id": 1804_i64 },
            "sender": { "player_id": 125861505_i64, "rally": true },
            "timeline": { "start_timestamp": 1772492175_i64, "end_timestamp": 1772492282_i64 },
            "opponents": [
                { "player_id": 56779522_i64, "attack": { "id": "640904" } },
                { "player_id": -2_i64, "attack": { "id": "ignored" } },
                { "player_id": 56779522_i64, "attack": { "id": "640903" } },
                { "player_id": 56779522_i64, "attack": { "id": "640904" } },
            ],
        };

        let key = build_report_dedupe_key(&document).expect("dedupe key");
        assert_eq!(
            key,
            "attacks:640903,640904|sender:125861505|server:1804|start:1772492175|end:1772492282"
        );
    }

    #[test]
    fn field_duel_reports_use_distinct_mail_keys() {
        let first = doc! {
            "metadata": { "mail_id": "46667158178428673715", "server_id": 1007_i64 },
            "sender": { "player_id": 147868539_i64 },
            "timeline": { "start_timestamp": 1784286616_i64, "end_timestamp": 1784286737_i64 },
            "opponents": [
                { "player_id": 153187088_i64, "attack": { "id": "273339901" } },
                { "player_id": 153187088_i64, "attack": { "id": "273340201" } },
                { "player_id": 153187088_i64, "attack": { "id": "273341101" } },
            ],
        };
        let mut second = first.clone();
        second.insert(
            "metadata",
            doc! {
                "mail_id": "46667147178428673715",
                "server_id": 1007_i64,
            },
        );

        let keys = [
            build_report_dedupe_key(&first).expect("first key"),
            build_report_dedupe_key(&second).expect("second key"),
        ];

        assert_eq!(keys, ["mail:46667158178428673715", "mail:46667147178428673715",]);
    }

    fn field_report(mail_id: &str) -> Document {
        let opponent = doc! {
            "player_id": 200,
            "tracking_key": "200_march_1",
            "attack": { "id": "attack-1" },
        };
        let mut second_opponent = opponent.clone();
        second_opponent.insert("attack", doc! { "id": "attack-2" });
        doc! {
            "metadata": { "mail_id": mail_id, "server_id": 1 },
            "sender": {
                "player_id": 100,
                "tracking_key": "100_march_1",
            },
            "timeline": { "start_timestamp": 100, "end_timestamp": 110 },
            "opponents": [opponent, second_opponent],
        }
    }

    #[test]
    fn identical_field_report_copies_share_a_key_regardless_of_entry_order() {
        let first = field_report("mail-1");
        let mut second = field_report("mail-2");
        second.get_array_mut("opponents").unwrap().reverse();
        let keys =
            [build_report_dedupe_key(&first).unwrap(), build_report_dedupe_key(&second).unwrap()];
        assert!(keys[0].starts_with("field:"));
        assert_eq!(keys.into_iter().collect::<std::collections::HashSet<_>>().len(), 1);
    }

    #[test]
    fn simultaneous_field_marches_with_the_same_attack_ids_remain_distinct() {
        let first = field_report("mail-1");
        let mut second = field_report("mail-2");
        second.get_document_mut("sender").unwrap().insert("tracking_key", "100_march_2");
        assert_ne!(build_report_dedupe_key(&first), build_report_dedupe_key(&second));
    }

    #[test]
    fn field_reports_with_different_context_remain_distinct() {
        let first = field_report("mail-1");
        for (section, field, value) in [
            ("metadata", "server_id", 2),
            ("sender", "player_id", 101),
            ("timeline", "start_timestamp", 101),
            ("timeline", "end_timestamp", 111),
        ] {
            let mut second = field_report("mail-2");
            second.get_document_mut(section).unwrap().insert(field, value);
            assert_ne!(
                build_report_dedupe_key(&first),
                build_report_dedupe_key(&second),
                "{field}"
            );
        }
    }

    #[test]
    fn field_reports_with_different_opponent_details_remain_distinct() {
        let first = field_report("mail-1");
        for (field, value) in [
            ("tracking_key", Bson::from("200_march_2")),
            ("player_id", Bson::from(201)),
            ("attack", Bson::from(doc! { "id": "attack-3" })),
        ] {
            let mut second = field_report("mail-2");
            second.get_array_mut("opponents").unwrap()[0]
                .as_document_mut()
                .unwrap()
                .insert(field, value);
            assert_ne!(
                build_report_dedupe_key(&first),
                build_report_dedupe_key(&second),
                "{field}"
            );
        }
    }

    #[test]
    fn field_dedupe_ignores_commanders_results_and_opponent_ticks() {
        let first = field_report("mail-1");
        let mut second = field_report("mail-2");
        second
            .get_document_mut("sender")
            .unwrap()
            .insert("commanders", doc! { "primary": { "id": 1 }, "secondary": { "id": 2 } });
        let opponent = second.get_array_mut("opponents").unwrap()[0].as_document_mut().unwrap();
        opponent.insert("commanders", doc! { "primary": { "id": 3 }, "secondary": { "id": 4 } });
        opponent.insert("start_tick", 10);
        opponent.insert("end_tick", 20);
        opponent.insert("battle_results", doc! { "sender": { "severely_wounded": 51 } });
        assert_eq!(build_report_dedupe_key(&first), build_report_dedupe_key(&second));
    }

    #[test]
    fn field_reports_preserve_partial_reports_and_repeated_entries() {
        let first = field_report("mail-1");
        let mut partial = field_report("mail-2");
        partial.get_array_mut("opponents").unwrap().pop();
        let mut repeated = field_report("mail-3");
        let opponents = repeated.get_array_mut("opponents").unwrap();
        opponents.push(opponents[0].clone());
        let keys =
            [&first, &partial, &repeated].map(|report| build_report_dedupe_key(report).unwrap());
        assert_eq!(keys.into_iter().collect::<std::collections::HashSet<_>>().len(), 3);
    }

    #[test]
    fn incomplete_field_reports_fall_back_to_mail_id() {
        for field in ["attack", "tracking_key", "player_id"] {
            let mut report = field_report("mail-1");
            report.get_array_mut("opponents").unwrap()[0].as_document_mut().unwrap().remove(field);
            assert_eq!(build_report_dedupe_key(&report).as_deref(), Some("mail:mail-1"), "{field}");
        }
        let mut report = field_report("mail-1");
        report.get_document_mut("sender").unwrap().insert("tracking_key", "");
        assert_eq!(build_report_dedupe_key(&report).as_deref(), Some("mail:mail-1"));
    }

    #[test]
    fn list_projection_includes_field_identity() {
        let projection = build_battle_list_projection();
        for field in ["sender.tracking_key", "opponents.tracking_key"] {
            assert_eq!(projection.get_i32(field), Ok(1), "{field}");
        }
    }

    #[test]
    fn garrison_participant_reports_share_combat_key() {
        let first = doc! {
            "metadata": { "mail_id": "mail-1", "server_id": 1007_i64 },
            "sender": { "player_id": 147868539_i64, "alliance_building_id": 1_i64 },
            "timeline": { "start_timestamp": 100_i64, "end_timestamp": 200_i64 },
            "opponents": [{ "player_id": 153187088_i64, "attack": { "id": "attack-1" } }],
        };
        let mut second = first.clone();
        second.insert("metadata", doc! { "mail_id": "mail-2", "server_id": 1007_i64 });

        assert_eq!(build_report_dedupe_key(&first), build_report_dedupe_key(&second));
    }

    #[test]
    fn structure_garrison_participant_reports_share_combat_key() {
        let first = doc! {
            "metadata": { "mail_id": "mail-1", "server_id": 1007_i64 },
            "sender": { "player_id": 147868539_i64 },
            "timeline": { "start_timestamp": 100_i64, "end_timestamp": 200_i64 },
            "opponents": [{
                "player_id": 153187088_i64,
                "structure_id": 25_i64,
                "attack": { "id": "attack-1" },
            }],
        };
        let mut second = first.clone();
        second.insert("metadata", doc! { "mail_id": "mail-2", "server_id": 1007_i64 });

        assert_eq!(build_report_dedupe_key(&first), build_report_dedupe_key(&second));
    }

    #[test]
    fn opponent_rally_participant_reports_share_combat_key() {
        let first = doc! {
            "metadata": { "mail_id": "mail-1", "server_id": 1007_i64 },
            "sender": { "player_id": 147868539_i64 },
            "timeline": { "start_timestamp": 100_i64, "end_timestamp": 200_i64 },
            "opponents": [{
                "player_id": 153187088_i64,
                "rally": true,
                "attack": { "id": "attack-1" },
            }],
        };
        let mut second = first.clone();
        second.insert("metadata", doc! { "mail_id": "mail-2", "server_id": 1007_i64 });

        assert_eq!(build_report_dedupe_key(&first), build_report_dedupe_key(&second));
    }

    #[test]
    fn dedupe_key_falls_back_to_mail_id_when_attack_ids_missing() {
        let document = doc! {
            "metadata": { "mail_id": "mail-1" },
            "sender": { "player_id": 1_i64 },
            "timeline": { "start_timestamp": 10_i64, "end_timestamp": 20_i64 },
            "opponents": [{ "player_id": 100_i64, "attack": { "id": "" } }],
        };

        let key = build_report_dedupe_key(&document).expect("dedupe key");
        assert_eq!(key, "mail:mail-1");
    }

    #[test]
    fn computes_trade_percent() {
        assert_eq!(compute_trade_percent(500, 250), 200);
        assert_eq!(compute_trade_percent(0, 0), 100);
        assert_eq!(compute_trade_percent(500, 0), 0);
    }

    #[test]
    fn prefers_garrison_opponent() {
        let opponents = [
            doc! {
                "player_id": 100,
                "start_tick": 1,
                "commanders": { "primary": { "id": 11 }, "secondary": { "id": 12 } },
            },
            doc! {
                "player_id": 101,
                "start_tick": 2,
                "alliance_building_id": 1,
                "commanders": { "primary": { "id": 21 }, "secondary": { "id": 22 } },
            },
        ];

        let opponent_refs = opponents.iter().collect::<Vec<_>>();
        let preferred = select_preferred_opponent(&opponent_refs).expect("preferred opponent");
        assert_eq!(nested_i64(preferred, &["player_id"]), Some(101));
    }

    #[test]
    fn fallback_summary_ignores_invalid_opponents() {
        let opponents = [
            doc! {
                "player_id": -2,
                "battle_results": {
                    "opponent": {
                        "troop_units": 999,
                        "dead": 999,
                        "severely_wounded": 999,
                        "slightly_wounded": 999,
                        "remaining": 999,
                        "kill_points": 999,
                    }
                }
            },
            doc! {
                "player_id": 100,
                "battle_results": {
                    "opponent": {
                        "troop_units": 10,
                        "dead": 20,
                        "severely_wounded": 30,
                        "slightly_wounded": 40,
                        "remaining": 50,
                        "kill_points": 60,
                    }
                }
            },
        ];

        let opponent_refs = opponents.iter().collect::<Vec<_>>();
        let summary = build_fallback_summary("opponent", &opponent_refs);
        assert_eq!(summary.troop_units, 10);
        assert_eq!(summary.dead, 20);
        assert_eq!(summary.severely_wounded, 30);
        assert_eq!(summary.slightly_wounded, 40);
        assert_eq!(summary.remaining, 50);
        assert_eq!(summary.kill_points, 60);
    }
}
