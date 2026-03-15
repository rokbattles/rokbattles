use mongodb::bson::{Bson, Document, doc};

use super::{
    query::DuelBattle2Request,
    types::{
        DuelBattle2Entry, DuelBattle2ListItem, DuelBattle2Participant, DuelBattle2RowWithCursor,
    },
};
use crate::bson_utils::{nested_document, nested_i64};

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
                            "sender": {
                                "primary_commander_id": "$sender.primary_commander.id",
                                "secondary_commander_id": "$sender.secondary_commander.id",
                            },
                            "opponent": {
                                "primary_commander_id": "$opponent.primary_commander.id",
                                "secondary_commander_id": "$opponent.secondary_commander.id",
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
        item: DuelBattle2ListItem {
            duel_id,
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
        secondary_commander_id: source
            .and_then(|value| nested_i64(value, &["secondary_commander_id"]))
            .unwrap_or(0),
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
}
