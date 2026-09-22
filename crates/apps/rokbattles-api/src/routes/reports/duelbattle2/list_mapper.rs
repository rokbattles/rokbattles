use mongodb::bson::{Bson, Document, doc};
use rokbattles_bson::{nested_bool, nested_document, nested_i64};

use super::{
    query::DuelBattle2Request,
    types::{
        DuelBattle2Entry, DuelBattle2ListItem, DuelBattle2Participant, DuelBattle2RowWithCursor,
    },
};
use crate::routes::reports::common::map_context::BattleMapContext;

pub(super) fn build_duelbattle2_list_pipeline(
    request: &DuelBattle2Request,
    fetch_limit: i64,
) -> Vec<Document> {
    let mut pipeline = vec![
        doc! {
            "$match": {
                "sender.duel.team_id": { "$exists": true, "$ne": Bson::Null },
                "metadata.mail_time": { "$exists": true, "$ne": Bson::Null },
            }
        },
        doc! {
            "$group": {
                "_id": "$sender.duel.team_id",
                "battles": { "$sum": 1 },
                "first_mail_time": { "$min": "$metadata.mail_time" },
                "latest_mail_time": { "$max": "$metadata.mail_time" },
                "win_streak": {
                    "$sum": {
                        "$cond": [{ "$eq": ["$battle_results.sender.win", true] }, 1, 0]
                    }
                },
                "opponent_kill_count": {
                    "$sum": {
                        "$add": [
                            { "$ifNull": ["$battle_results.opponent.severely_wounded", 0] },
                            { "$ifNull": ["$battle_results.opponent.dead", 0] },
                        ]
                    }
                },
                "sender_kill_points": { "$sum": { "$ifNull": ["$battle_results.sender.kill_points", 0] } },
                "opponent_kill_points": { "$sum": { "$ifNull": ["$battle_results.opponent.kill_points", 0] } },
            }
        },
    ];

    if let Some(before_cursor) = request.before_cursor {
        pipeline.push(doc! {
            "$match": {
                "$expr": { "$gt": ["$latest_mail_time", before_cursor] }
            }
        });
    } else if let Some(after_cursor) = request.after_cursor {
        pipeline.push(doc! {
            "$match": {
                "$expr": { "$lt": ["$latest_mail_time", after_cursor] }
            }
        });
    }

    pipeline.extend([
        doc! { "$sort": { "latest_mail_time": request.sort_direction() } },
        doc! { "$limit": fetch_limit },
        doc! {
            "$lookup": {
                "from": "mails_duelbattle2",
                "let": { "duel_id": "$_id", "first_mail_time": "$first_mail_time" },
                "pipeline": [
                    {
                        "$match": {
                            "$expr": {
                                "$and": [
                                    { "$eq": ["$sender.duel.team_id", "$$duel_id"] },
                                    { "$eq": ["$metadata.mail_time", "$$first_mail_time"] },
                                ]
                            }
                        }
                    },
                    { "$sort": { "metadata.mail_time": 1 } },
                    {
                        "$project": {
                            "metadata.server_id": 1,
                            "sender": {
                                "primary_commander_id": "$sender.primary_commander.id",
                                "primary_commander_awakened": "$sender.primary_commander.awakened",
                                "secondary_commander_id": "$sender.secondary_commander.id",
                                "secondary_commander_awakened": "$sender.secondary_commander.awakened",
                            },
                            "opponent": {
                                "primary_commander_id": "$opponent.primary_commander.id",
                                "primary_commander_awakened": "$opponent.primary_commander.awakened",
                                "secondary_commander_id": "$opponent.secondary_commander.id",
                                "secondary_commander_awakened": "$opponent.secondary_commander.awakened",
                            },
                        }
                    },
                    { "$limit": 1 },
                ],
                "as": "first_doc",
            }
        },
        doc! { "$unwind": "$first_doc" },
    ]);

    pipeline
}

pub(super) fn map_duelbattle2_list_document(
    document: &Document,
) -> Option<DuelBattle2RowWithCursor> {
    let duel_id = nested_i64(document, &["_id"])?;
    let first_mail_time = nested_i64(document, &["first_mail_time"])?;
    let latest_mail_time = nested_i64(document, &["latest_mail_time"])?;
    let first_doc = nested_document(document, &["first_doc"])?;

    let sender = map_participant(first_doc, "sender");
    let opponent = map_participant(first_doc, "opponent");

    let sender_kill_points = nested_i64(document, &["sender_kill_points"]).unwrap_or(0);
    let opponent_kill_points = nested_i64(document, &["opponent_kill_points"]).unwrap_or(0);

    Some(DuelBattle2RowWithCursor {
        latest_mail_time,
        map_context: BattleMapContext::from_server_id(
            nested_i64(first_doc, &["metadata", "server_id"]),
            first_mail_time,
        ),
        item: DuelBattle2ListItem {
            duel_id,
            battles: nested_i64(document, &["battles"]).unwrap_or(1),
            kvk_mapcode: "Unknown".into(),
            kvk_banner: None,
            win_streak: nested_i64(document, &["win_streak"]).unwrap_or(0),
            mail_time: first_mail_time,
            kill_count: nested_i64(document, &["opponent_kill_count"]).unwrap_or(0),
            trade_percent: compute_trade_percent(sender_kill_points, opponent_kill_points),
            entry: DuelBattle2Entry { sender, opponent },
        },
    })
}

fn map_participant(document: &Document, key: &str) -> DuelBattle2Participant {
    let source = nested_document(document, &[key]);

    DuelBattle2Participant {
        primary_commander_id: source
            .and_then(|value| nested_i64(value, &["primary_commander_id"]))
            .unwrap_or(0),
        primary_commander_awakened: source
            .and_then(|value| nested_bool(value, &["primary_commander_awakened"])),
        secondary_commander_id: source
            .and_then(|value| nested_i64(value, &["secondary_commander_id"]))
            .unwrap_or(0),
        secondary_commander_awakened: source
            .and_then(|value| nested_bool(value, &["secondary_commander_awakened"])),
    }
}

fn compute_trade_percent(sender_kill_points: i64, opponent_kill_points: i64) -> i64 {
    if sender_kill_points == opponent_kill_points {
        return 100;
    }

    if opponent_kill_points <= 0 {
        return 0;
    }

    ((sender_kill_points as f64 / opponent_kill_points as f64) * 100.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::compute_trade_percent;

    #[test]
    fn computes_trade_percent() {
        assert_eq!(compute_trade_percent(250, 250), 100);
        assert_eq!(compute_trade_percent(500, 250), 200);
        assert_eq!(compute_trade_percent(500, 0), 0);
    }

    fn arena_document(server_id: Option<i64>) -> mongodb::bson::Document {
        let mut metadata = mongodb::bson::doc! {};
        if let Some(server_id) = server_id {
            metadata.insert("server_id", server_id);
        }
        mongodb::bson::doc! {
            "_id": 42,
            "first_mail_time": 1_789_929_193_000_000_i64,
            "latest_mail_time": 1_789_929_300_000_000_i64,
            "battles": 5,
            "win_streak": 3,
            "opponent_kill_count": 12345,
            "sender_kill_points": 500,
            "opponent_kill_points": 250,
            "first_doc": { "metadata": metadata },
        }
    }

    #[test]
    fn arena_uses_first_report_server_and_time_without_a_kvk_flag() {
        use mongodb::bson::doc;
        let row = super::map_duelbattle2_list_document(&arena_document(Some(16053)))
            .expect("valid arena row");
        let snapshots = [
            doc! {
                "Id": 16053, "Order": 13249, "Mode": 16,
                "OpenTime": 1_789_000_000_000_i64, "CloseTime": 1_789_929_250_000_i64,
                "MapName": "LostLand_Map_4_2_v2",
            },
            doc! {
                "Id": 16053, "Order": 13300, "Mode": 16,
                "OpenTime": 1_789_929_250_000_i64, "CloseTime": 1_790_000_000_000_i64,
                "MapName": "LostLand_Map_20_v2",
            },
        ];
        assert_eq!(
            row.map_context.resolve(&snapshots),
            (
                "#C13249".into(),
                Some("https://cdn.rokbattles.com/game/banners/s4heroic_anthem_cover.png".into())
            )
        );
    }

    #[test]
    fn arena_missing_invalid_or_unmatched_server_stays_unknown() {
        for server_id in [None, Some(0), Some(-1), Some(16053)] {
            let row = super::map_duelbattle2_list_document(&arena_document(server_id))
                .expect("missing context must not discard a duel");
            assert_eq!(row.map_context.resolve(&[]), ("Unknown".into(), None));
        }
    }

    #[test]
    fn arena_response_preserves_totals_and_timestamps_without_exposing_lookup_fields() {
        let row = super::map_duelbattle2_list_document(&arena_document(Some(16053)))
            .expect("valid arena row");
        assert_eq!(row.latest_mail_time, 1_789_929_300_000_000);
        let value = serde_json::to_value(row.item).expect("serializable row");
        assert_eq!(value["battles"], 5);
        assert_eq!(value["winStreak"], 3);
        assert_eq!(value["killCount"], 12345);
        assert_eq!(value["tradePercent"], 200);
        assert_eq!(value["mailTime"], 1_789_929_193_000_000_i64);
        assert_eq!(value["kvkMapcode"], "Unknown");
        assert!(value["kvkBanner"].is_null());
        for private in ["serverId", "metadata", "mapContext"] {
            assert!(value.get(private).is_none(), "leaked {private}");
        }
    }

    #[test]
    fn arena_lookup_projects_server_from_the_first_report() {
        let request = super::DuelBattle2Request { before_cursor: None, after_cursor: None };
        let pipeline = super::build_duelbattle2_list_pipeline(&request, 101);
        let lookup = pipeline
            .iter()
            .find_map(|stage| stage.get_document("$lookup").ok())
            .expect("first report lookup");
        let stages = lookup.get_array("pipeline").expect("lookup stages");
        let projection = stages
            .iter()
            .filter_map(|stage| stage.as_document())
            .find_map(|stage| stage.get_document("$project").ok())
            .expect("lookup projection");
        assert_eq!(projection.get_i32("metadata.server_id"), Ok(1));
    }

    #[test]
    fn arena_counts_all_reports_in_a_duel_including_losses() {
        let request = super::DuelBattle2Request { before_cursor: None, after_cursor: None };
        let pipeline = super::build_duelbattle2_list_pipeline(&request, 101);
        let group = pipeline
            .iter()
            .find_map(|stage| stage.get_document("$group").ok())
            .expect("duel grouping");
        assert_eq!(
            group.get_document("battles").expect("battle count"),
            &mongodb::bson::doc! { "$sum": 1 }
        );
    }
}
