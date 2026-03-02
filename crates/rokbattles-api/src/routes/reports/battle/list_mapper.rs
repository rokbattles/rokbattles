use std::cmp::Ordering;

use mongodb::bson::{Bson, Document, doc};

use crate::bson_utils::{nested_document, nested_i64, nested_str};

use super::query::ReportsRequest;
use super::types::{
    ReportListItem, ReportListParticipant, ReportRowWithCursor, ReportSummary, ReportSummaryEntry,
    ReportTimeline, TimelineSample,
};

const INVALID_OPPONENT_PLAYER_IDS: [i64; 2] = [-2, 0];

pub(crate) fn build_battle_list_projection() -> Document {
    doc! {
        "metadata.mail_id": 1,
        "metadata.mail_time": 1,
        "metadata.server_id": 1,
        "timeline.start_timestamp": 1,
        "timeline.end_timestamp": 1,
        "timeline.sampling.tick": 1,
        "timeline.sampling.count": 1,
        "sender.player_id": 1,
        "sender.commanders.primary.id": 1,
        "sender.commanders.secondary.id": 1,
        "summary.sender.kill_points": 1,
        "summary.sender.dead": 1,
        "summary.sender.severely_wounded": 1,
        "summary.sender.slightly_wounded": 1,
        "summary.sender.remaining": 1,
        "summary.sender.troop_units": 1,
        "summary.opponent.kill_points": 1,
        "summary.opponent.dead": 1,
        "summary.opponent.severely_wounded": 1,
        "summary.opponent.slightly_wounded": 1,
        "summary.opponent.remaining": 1,
        "summary.opponent.troop_units": 1,
        "opponents.player_id": 1,
        "opponents.attack.id": 1,
        "opponents.start_tick": 1,
        "opponents.alliance_building_id": 1,
        "opponents.structure_id": 1,
        "opponents.commanders.primary.id": 1,
        "opponents.commanders.secondary.id": 1,
        "opponents.battle_results.sender.kill_points": 1,
        "opponents.battle_results.sender.dead": 1,
        "opponents.battle_results.sender.severely_wounded": 1,
        "opponents.battle_results.sender.slightly_wounded": 1,
        "opponents.battle_results.sender.remaining": 1,
        "opponents.battle_results.sender.troop_units": 1,
        "opponents.battle_results.opponent.kill_points": 1,
        "opponents.battle_results.opponent.dead": 1,
        "opponents.battle_results.opponent.severely_wounded": 1,
        "opponents.battle_results.opponent.slightly_wounded": 1,
        "opponents.battle_results.opponent.remaining": 1,
        "opponents.battle_results.opponent.troop_units": 1,
    }
}

pub(super) fn build_battle_list_pipeline(
    request: &ReportsRequest,
    reports_match: Document,
    fetch_limit: i64,
) -> Vec<Document> {
    let mut pipeline = vec![
        doc! { "$match": reports_match },
        // Sort first so Mongo can use the mail-time index before we build dedupe fields.
        doc! { "$sort": { "metadata.mail_time": -1 } },
        // Keep only list fields here so `$group` doesn't carry full report payloads.
        doc! { "$project": build_battle_list_projection() },
        doc! {
            "$addFields": {
                "_dedupe_key": {
                    "attack_ids": {
                        "$sortArray": {
                            "input": {
                                "$setUnion": [
                                    {
                                        "$map": {
                                            "input": {
                                                "$filter": {
                                                    "input": { "$ifNull": ["$opponents", []] },
                                                    "as": "opponent",
                                                    "cond": {
                                                        "$and": [
                                                            {
                                                                "$not": [
                                                                    {
                                                                        "$in": [
                                                                            { "$ifNull": ["$$opponent.player_id", 0] },
                                                                            [-2, 0]
                                                                        ]
                                                                    }
                                                                ]
                                                            },
                                                            { "$ne": [{ "$ifNull": ["$$opponent.attack.id", ""] }, ""] }
                                                        ]
                                                    }
                                                }
                                            },
                                            "as": "opponent",
                                            "in": { "$ifNull": ["$$opponent.attack.id", ""] }
                                        }
                                    },
                                    []
                                ]
                            },
                            "sortBy": 1
                        }
                    },
                    "sender_player_id": { "$ifNull": ["$sender.player_id", 0] },
                    "server_id": { "$ifNull": ["$metadata.server_id", 0] },
                    "start_timestamp": { "$ifNull": ["$timeline.start_timestamp", 0] },
                    "end_timestamp": { "$ifNull": ["$timeline.end_timestamp", 0] },
                }
            }
        },
        doc! {
            "$group": {
                "_id": "$_dedupe_key",
                "latest_mail_time": { "$first": "$metadata.mail_time" },
                "document": { "$first": "$$ROOT" },
            }
        },
    ];

    if let Some(before_cursor) = request.before_cursor {
        pipeline.push(doc! { "$match": { "latest_mail_time": { "$gt": before_cursor } } });
    } else if let Some(after_cursor) = request.after_cursor {
        pipeline.push(doc! { "$match": { "latest_mail_time": { "$lt": after_cursor } } });
    }

    pipeline.extend([
        doc! { "$sort": { "latest_mail_time": request.sort_direction() } },
        doc! { "$limit": fetch_limit },
        doc! { "$replaceRoot": { "newRoot": "$document" } },
    ]);

    pipeline
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
        time_start,
        time_end,
        sender: ReportListParticipant {
            primary_commander_id: nested_i64(document, &["sender", "commanders", "primary", "id"])
                .unwrap_or(0),
            secondary_commander_id: nested_i64(
                document,
                &["sender", "commanders", "secondary", "id"],
            )
            .unwrap_or(0),
        },
        opponent: ReportListParticipant {
            primary_commander_id: nested_i64(preferred_opponent, &["commanders", "primary", "id"])
                .unwrap_or(0),
            secondary_commander_id: nested_i64(
                preferred_opponent,
                &["commanders", "secondary", "id"],
            )
            .unwrap_or(0),
        },
        battles,
        kill_count,
        trade_percent,
        summary: ReportSummary {
            sender: sender_summary,
            opponent: opponent_summary,
        },
        timeline: ReportTimeline {
            start_timestamp: nested_i64(document, &["timeline", "start_timestamp"])
                .unwrap_or(time_start),
            end_timestamp: nested_i64(document, &["timeline", "end_timestamp"]).unwrap_or(time_end),
            sampling: extract_timeline_samples(document),
        },
    };

    Some(ReportRowWithCursor { mail_time, item })
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

fn extract_opponents(document: &Document) -> Vec<Document> {
    document
        .get_array("opponents")
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Bson::as_document)
        .cloned()
        .collect()
}

fn get_valid_sorted_opponents(opponents: &[Document]) -> Vec<Document> {
    let mut valid = opponents
        .iter()
        .filter(|opponent| is_valid_opponent(opponent))
        .cloned()
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
    !INVALID_OPPONENT_PLAYER_IDS.contains(&player_id)
}

fn select_preferred_opponent(opponents: &[Document]) -> Option<&Document> {
    for opponent in opponents {
        if has_non_null_field(opponent, "alliance_building_id")
            || has_non_null_field(opponent, "structure_id")
        {
            return Some(opponent);
        }
    }

    opponents.first()
}

fn has_non_null_field(document: &Document, key: &str) -> bool {
    matches!(document.get(key), Some(value) if value.as_null().is_none())
}

fn resolve_summary_entry(
    document: &Document,
    side: &str,
    opponents: &[Document],
) -> ReportSummaryEntry {
    let fallback = build_fallback_summary(side, opponents);
    let source = nested_document(document, &["summary", side]);

    ReportSummaryEntry {
        troop_units: source
            .and_then(|value| nested_i64(value, &["troop_units"]))
            .unwrap_or(fallback.troop_units),
        dead: source
            .and_then(|value| nested_i64(value, &["dead"]))
            .unwrap_or(fallback.dead),
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

fn build_fallback_summary(side: &str, opponents: &[Document]) -> ReportSummaryEntry {
    let mut summary = ReportSummaryEntry {
        troop_units: 0,
        dead: 0,
        severely_wounded: 0,
        slightly_wounded: 0,
        remaining: 0,
        kill_points: 0,
    };

    for opponent in opponents
        .iter()
        .filter(|opponent| is_valid_opponent(opponent))
    {
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
    use crate::routes::reports::battle::query::ReportsFilterSide;

    fn empty_request() -> ReportsRequest {
        ReportsRequest {
            before_cursor: None,
            after_cursor: None,
            filter_type: None,
            player_id: None,
            sender_primary_commander_id: None,
            sender_secondary_commander_id: None,
            opponent_primary_commander_id: None,
            opponent_secondary_commander_id: None,
            rally_side: ReportsFilterSide::None,
            garrison_side: ReportsFilterSide::None,
            garrison_building_type: None,
        }
    }

    #[test]
    fn battle_list_pipeline_groups_by_dedupe_key() {
        let request = empty_request();
        let pipeline = build_battle_list_pipeline(&request, doc! { "metadata.kvk": true }, 101);

        let stage_names = pipeline
            .iter()
            .filter_map(|stage| stage.keys().next().cloned())
            .collect::<Vec<_>>();
        assert_eq!(
            stage_names,
            vec![
                "$match",
                "$sort",
                "$project",
                "$addFields",
                "$group",
                "$sort",
                "$limit",
                "$replaceRoot",
            ]
        );

        let group_stage = pipeline[4].get_document("$group").expect("$group stage");
        assert_eq!(
            group_stage.get_str("_id").expect("group id"),
            "$_dedupe_key"
        );

        let add_fields_stage = pipeline[3]
            .get_document("$addFields")
            .expect("$addFields stage");
        let dedupe_key = add_fields_stage
            .get_document("_dedupe_key")
            .expect("_dedupe_key");
        assert!(dedupe_key.get("attack_ids").is_some());
        assert!(dedupe_key.get("sender_player_id").is_some());
        assert!(dedupe_key.get("server_id").is_some());
        assert!(dedupe_key.get("start_timestamp").is_some());
        assert!(dedupe_key.get("end_timestamp").is_some());
    }

    #[test]
    fn battle_list_pipeline_applies_after_cursor_post_dedupe() {
        let mut request = empty_request();
        request.after_cursor = Some(123);
        let pipeline = build_battle_list_pipeline(&request, doc! {}, 101);

        let cursor_match = pipeline[5]
            .get_document("$match")
            .expect("cursor match stage");
        let latest_mail_time = cursor_match
            .get_document("latest_mail_time")
            .expect("latest_mail_time match");
        assert_eq!(latest_mail_time.get_i64("$lt").expect("$lt"), 123);
    }

    #[test]
    fn computes_trade_percent() {
        assert_eq!(compute_trade_percent(500, 250), 200);
        assert_eq!(compute_trade_percent(0, 0), 100);
        assert_eq!(compute_trade_percent(500, 0), 0);
    }

    #[test]
    fn prefers_garrison_opponent() {
        let opponents = vec![
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

        let preferred = select_preferred_opponent(&opponents).expect("preferred opponent");
        assert_eq!(nested_i64(preferred, &["player_id"]), Some(101));
    }

    #[test]
    fn fallback_summary_ignores_invalid_opponents() {
        let opponents = vec![
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

        let summary = build_fallback_summary("opponent", &opponents);
        assert_eq!(summary.troop_units, 10);
        assert_eq!(summary.dead, 20);
        assert_eq!(summary.severely_wounded, 30);
        assert_eq!(summary.slightly_wounded, 40);
        assert_eq!(summary.remaining, 50);
        assert_eq!(summary.kill_points, 60);
    }
}
