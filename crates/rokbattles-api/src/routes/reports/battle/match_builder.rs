use mongodb::bson::{Bson, Document, doc};

use super::query::{
    ReportsFilterSide, ReportsFilterType, ReportsGarrisonBuildingType, ReportsRequest,
};

/// Build the MongoDB `$match` object for battle report list queries.
pub(crate) fn build_reports_match(request: &ReportsRequest) -> Document {
    let mut match_pipeline: Vec<Document> = vec![doc! {
        "opponents": {
            "$elemMatch": { "player_id": { "$nin": [-2, 0] } }
        }
    }];

    if let Some(before_cursor) = request.before_cursor {
        match_pipeline.push(doc! { "metadata.mail_time": { "$gt": before_cursor } });
    } else if let Some(after_cursor) = request.after_cursor {
        match_pipeline.push(doc! { "metadata.mail_time": { "$lt": after_cursor } });
    }

    if let Some(player_id) = request.player_id {
        match_pipeline.push(doc! {
            "$or": [
                { "sender.player_id": player_id },
                { "opponents.player_id": player_id },
            ]
        });
    }

    if let Some(commander_id) = request.sender_primary_commander_id {
        match_pipeline.push(doc! { "sender.commanders.primary.id": commander_id });
    }

    if let Some(commander_id) = request.sender_secondary_commander_id {
        match_pipeline.push(doc! { "sender.commanders.secondary.id": commander_id });
    }

    if let Some(commander_id) = request.opponent_primary_commander_id {
        match_pipeline.push(doc! {
            "opponents": {
                "$elemMatch": {
                    "player_id": { "$nin": [-2, 0] },
                    "commanders.primary.id": commander_id,
                }
            }
        });
    }

    if let Some(commander_id) = request.opponent_secondary_commander_id {
        match_pipeline.push(doc! {
            "opponents": {
                "$elemMatch": {
                    "player_id": { "$nin": [-2, 0] },
                    "commanders.secondary.id": commander_id,
                }
            }
        });
    }

    if let Some(filter_type) = request.filter_type {
        match filter_type {
            ReportsFilterType::Kvk => {
                match_pipeline.push(doc! { "metadata.kvk": true });
            }
            ReportsFilterType::Ark => {
                match_pipeline.push(doc! { "metadata.mail_role": "dungeon" });
            }
            ReportsFilterType::Home => {
                match_pipeline.push(doc! {
                    "$and": [
                        { "metadata.kvk": { "$ne": true } },
                        { "metadata.mail_role": { "$ne": "dungeon" } },
                        {
                            "$or": [
                                { "sender.supreme_strife.battle_id": { "$in": [Bson::Null, Bson::String("".to_string())] } },
                                { "sender.supreme_strife.team_id": { "$in": [Bson::Null, Bson::Int32(0), Bson::Int64(0)] } },
                            ]
                        }
                    ]
                });
            }
            ReportsFilterType::Strife => {
                match_pipeline.push(doc! {
                    "$and": [
                        { "sender.supreme_strife.battle_id": { "$exists": true, "$nin": [Bson::Null, Bson::String("".to_string())] } },
                        { "sender.supreme_strife.team_id": { "$exists": true, "$nin": [Bson::Null, Bson::Int32(0), Bson::Int64(0)] } },
                    ]
                });
            }
        }
    }

    let mut rally_conditions: Vec<Document> = Vec::new();
    if matches!(
        request.rally_side,
        ReportsFilterSide::Sender | ReportsFilterSide::Both
    ) {
        rally_conditions
            .push(doc! { "sender.rally": { "$in": [Bson::Int32(1), Bson::Boolean(true)] } });
    }
    if matches!(
        request.rally_side,
        ReportsFilterSide::Opponent | ReportsFilterSide::Both
    ) {
        rally_conditions.push(doc! {
            "opponents": {
                "$elemMatch": {
                    "player_id": { "$nin": [-2, 0] },
                    "rally": { "$in": [Bson::Int32(1), Bson::Boolean(true)] },
                }
            }
        });
    }
    append_compound_condition(&mut match_pipeline, rally_conditions);

    let mut garrison_conditions: Vec<Document> = Vec::new();
    if matches!(
        request.garrison_side,
        ReportsFilterSide::Sender | ReportsFilterSide::Both
    ) {
        garrison_conditions.push(build_garrison_field_condition(
            "sender.alliance_building_id",
            request.garrison_building_type,
        ));
    }
    if matches!(
        request.garrison_side,
        ReportsFilterSide::Opponent | ReportsFilterSide::Both
    ) {
        garrison_conditions.push(build_opponent_garrison_condition(
            request.garrison_building_type,
        ));
    }
    append_compound_condition(&mut match_pipeline, garrison_conditions);

    if match_pipeline.len() == 1 {
        match_pipeline.into_iter().next().unwrap_or_default()
    } else {
        doc! { "$and": match_pipeline }
    }
}

fn append_compound_condition(target: &mut Vec<Document>, conditions: Vec<Document>) {
    if conditions.is_empty() {
        return;
    }

    if conditions.len() == 1 {
        if let Some(condition) = conditions.into_iter().next() {
            target.push(condition);
        }
        return;
    }

    target.push(doc! { "$or": conditions });
}

fn build_garrison_field_condition(
    path: &str,
    garrison_type: Option<ReportsGarrisonBuildingType>,
) -> Document {
    let mut condition = Document::new();
    let value = match garrison_type {
        Some(ReportsGarrisonBuildingType::Flag) => Bson::Int32(1),
        Some(ReportsGarrisonBuildingType::Fortress) => Bson::Int32(3),
        Some(ReportsGarrisonBuildingType::Other) => Bson::Document(
            doc! { "$exists": true, "$nin": [Bson::Int32(1), Bson::Int32(3), Bson::Null] },
        ),
        None => Bson::Document(doc! { "$exists": true, "$ne": Bson::Null }),
    };
    condition.insert(path, value);
    condition
}

fn build_opponent_garrison_condition(
    garrison_type: Option<ReportsGarrisonBuildingType>,
) -> Document {
    let elem_match = match garrison_type {
        Some(ReportsGarrisonBuildingType::Flag) => {
            doc! { "player_id": { "$nin": [-2, 0] }, "alliance_building_id": 1 }
        }
        Some(ReportsGarrisonBuildingType::Fortress) => {
            doc! { "player_id": { "$nin": [-2, 0] }, "alliance_building_id": 3 }
        }
        Some(ReportsGarrisonBuildingType::Other) => doc! {
            "player_id": { "$nin": [-2, 0] },
            "alliance_building_id": { "$exists": true, "$nin": [Bson::Int32(1), Bson::Int32(3), Bson::Null] }
        },
        None => doc! {
            "player_id": { "$nin": [-2, 0] },
            "alliance_building_id": { "$exists": true, "$ne": Bson::Null }
        },
    };

    doc! {
        "opponents": {
            "$elemMatch": elem_match,
        }
    }
}
