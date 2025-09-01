use crate::AppState;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use futures_util::TryStreamExt;
use mongodb::{
    bson::{Document, doc},
    options::AggregateOptions,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ListParams {
    #[serde(default)]
    limit: Option<usize>,

    #[serde(default)]
    after: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiEntry {
    hash: String,
    self_commander_id: i64,
    self_secondary_commander_id: i64,
    enemy_commander_id: i64,
    enemy_secondary_commander_id: i64,
    start_date: i64,
}

#[derive(Debug, Serialize)]
pub struct ApiGroup {
    hash: String,
    entries: Vec<ApiEntry>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse {
    items: Vec<ApiGroup>,
    next_cursor: Option<String>,
    count: usize,
}

fn parse_cursor(after: Option<&str>) -> (Option<i64>, Option<String>) {
    if let Some(cur) = after {
        let mut parts = cur.splitn(2, ':');
        let ts = parts.next().and_then(|s| s.parse::<i64>().ok());
        let ph = parts.next().map(|s| s.to_string());
        (ts, ph)
    } else {
        (None, None)
    }
}

pub async fn list_reports(
    State(st): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<ApiResponse>, StatusCode> {
    let col = st.db.collection::<Document>("battleReports");

    // 1-100 (default 100)
    let limit = params.limit.unwrap_or(100).clamp(1, 100) as i64;
    let (cursor_ts, cursor_ph) = parse_cursor(params.after.as_deref());

    let mut pipeline = vec![
        doc! { "$match": { "report.enemy.player_id": { "$ne": -2 } } },
        doc! { "$project": {
            "entryHash": "$metadata.hash",
            "parentHash": "$metadata.parentHash",
            "startDate": "$report.metadata.start_date",
            "selfCommanderId": "$report.self.primary_commander.id",
            "selfSecondaryCommanderId": "$report.self.secondary_commander.id",
            "enemyCommanderId": "$report.enemy.primary_commander.id",
            "enemySecondaryCommanderId": "$report.enemy.secondary_commander.id",
        }},
        doc! { "$sort": { "startDate": -1_i32 } },
        doc! { "$group": {
            "_id": "$parentHash",
            "latestStart": { "$first": "$startDate" },
            "entry": { "$first": {
                "hash": "$entryHash",
                "self_commander_id": "$selfCommanderId",
                "self_secondary_commander_id": "$selfSecondaryCommanderId",
                "enemy_commander_id": "$enemyCommanderId",
                "enemy_secondary_commander_id": "$enemySecondaryCommanderId",
                "start_date": "$startDate",
            }},
        }},
    ];

    if let (Some(ts), Some(ph)) = (cursor_ts, cursor_ph.as_deref()) {
        pipeline.push(doc! { "$match": { "$expr": {
            "$or": [
                { "$lt": [ "$latestStart", ts ] },
                { "$and": [
                    { "$eq": [ "$latestStart", ts ] },
                    { "$lt": [ "$_id", ph ] }
                ]}
            ]
        } }});
    }

    pipeline.push(doc! { "$sort": { "latestStart": -1_i32, "_id": -1_i32 } });
    pipeline.push(doc! { "$limit": limit });

    let opts = AggregateOptions::builder().allow_disk_use(true).build();
    let mut cursor = col
        .aggregate(pipeline)
        .with_options(opts)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut items: Vec<ApiGroup> = Vec::new();
    let mut last_cursor_ts: Option<i64> = None;
    let mut last_cursor_ph: Option<String> = None;

    while let Some(doc) = cursor
        .try_next()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        let parent_hash = doc.get_str("_id").unwrap_or_default().to_string();
        let latest_start = doc.get_i64("latestStart").unwrap_or(0);

        let entry_doc = doc.get_document("entry").ok();
        let (hash, self_id, self_second_id, enemy_id, enemy_second_id, start_date) =
            if let Some(ed) = entry_doc {
                (
                    ed.get_str("hash").unwrap_or_default().to_string(),
                    ed.get_i32("self_commander_id").unwrap_or(0) as i64,
                    ed.get_i32("self_secondary_commander_id").unwrap_or(0) as i64,
                    ed.get_i32("enemy_commander_id").unwrap_or(0) as i64,
                    ed.get_i32("enemy_secondary_commander_id").unwrap_or(0) as i64,
                    ed.get_i64("start_date").unwrap_or(latest_start),
                )
            } else {
                (String::new(), 0, 0, 0, 0, latest_start)
            };

        items.push(ApiGroup {
            hash: parent_hash.clone(),
            entries: vec![ApiEntry {
                hash,
                self_commander_id: self_id,
                self_secondary_commander_id: self_second_id,
                enemy_commander_id: enemy_id,
                enemy_secondary_commander_id: enemy_second_id,
                start_date,
            }],
        });

        last_cursor_ts = Some(latest_start);
        last_cursor_ph = Some(parent_hash);
    }

    let next_cursor = match (last_cursor_ts, last_cursor_ph) {
        (Some(ts), Some(ph)) if !ph.is_empty() => Some(format!("{ts}:{ph}")),
        _ => None,
    };

    let count = items.len();
    Ok(Json(ApiResponse {
        items,
        next_cursor,
        count,
    }))
}
