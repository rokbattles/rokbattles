use crate::AppState;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use futures_util::TryStreamExt;
use mongodb::{
    bson,
    bson::{Bson, Document, doc},
    options::AggregateOptions,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue};

#[derive(Debug, Deserialize)]
pub struct DetailParams {
    #[serde(default)]
    limit: Option<usize>,

    #[serde(default)]
    after: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiReportEntry {
    hash: String,
    report: JsonValue,
    start_date: i64,
}

#[derive(Debug, Serialize, Default)]
pub struct ApiReportSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    battle_results: Option<JsonValue>,
}

#[derive(Debug, Serialize)]
pub struct ApiDetailResponse {
    parent_hash: String,
    items: Vec<ApiReportEntry>,
    next_cursor: Option<String>,
    count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    report: Option<ApiReportSummary>,
}

fn parse_cursor(after: Option<&str>) -> (Option<i64>, Option<String>) {
    if let Some(cur) = after {
        let mut parts = cur.splitn(2, ':');
        let ts = parts.next().and_then(|s| s.parse::<i64>().ok());
        let h = parts.next().map(|s| s.to_string());
        (ts, h)
    } else {
        (None, None)
    }
}

fn doc_to_json(doc: &Document) -> JsonValue {
    bson::from_bson::<JsonValue>(Bson::Document(doc.clone())).unwrap_or(JsonValue::Null)
}

pub async fn report_by_parent(
    State(st): State<AppState>,
    Path(parent_hash): Path<String>,
    Query(params): Query<DetailParams>,
) -> Result<Json<ApiDetailResponse>, StatusCode> {
    let col = st.db.collection::<Document>("battleReports");

    let limit = params.limit.unwrap_or(50).clamp(1, 200) as i64;
    let (cursor_ts, cursor_hash) = parse_cursor(params.after.as_deref());

    let base_filter = doc! {
        "metadata.parentHash": &parent_hash,
        "report.enemy.player_id": { "$ne": -2 }
    };

    let mut pipeline = vec![
        doc! { "$match": base_filter.clone() },
        doc! { "$project": {
            "hash": "$metadata.hash",
            "report": "$report",
            "startDate": "$report.metadata.start_date",
        }},
        doc! { "$sort": { "startDate": 1_i32, "hash": 1_i32 }},
    ];

    if let (Some(ts), Some(h)) = (cursor_ts, cursor_hash.as_deref()) {
        pipeline.push(doc! { "$match": { "$expr": {
            "$or": [
                { "$lt": [ "$startDate", ts ] },
                { "$and": [
                    { "$eq": [ "$startDate", ts ] },
                    { "$lt": [ "$hash", h ] }
                ]}
            ]
        } }});
    }

    pipeline.push(doc! { "$limit": limit });

    let opts = AggregateOptions::builder().allow_disk_use(true).build();
    let mut cursor = col
        .aggregate(pipeline)
        .with_options(opts)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut items: Vec<ApiReportEntry> = Vec::new();
    let mut last_ts: Option<i64> = None;
    let mut last_hash: Option<String> = None;

    while let Some(doc) = cursor
        .try_next()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        let hash = doc.get_str("hash").unwrap_or_default().to_string();
        let start_date = doc.get_i64("startDate").unwrap_or(0);

        let report_val = match doc.get("report") {
            Some(Bson::Document(d)) => doc_to_json(d),
            _ => JsonValue::Null,
        };

        items.push(ApiReportEntry {
            hash: hash.clone(),
            report: report_val,
            start_date,
        });

        last_ts = Some(start_date);
        last_hash = Some(hash);
    }

    let next_cursor = match (last_ts, last_hash) {
        (Some(ts), Some(h)) if !h.is_empty() => Some(format!("{ts}:{h}")),
        _ => None,
    };

    let count = items.len();

    let mut combined_results: JsonMap<String, JsonValue> = JsonMap::new();

    let final_pipeline = vec![
        doc! { "$match": base_filter.clone() },
        doc! { "$sort": { "report.metadata.start_date": -1_i32, "metadata.hash": -1_i32 } },
        doc! { "$limit": 1 },
        doc! { "$project": { "battle_results": "$report.battle_results" } },
    ];

    let final_opts = AggregateOptions::builder().allow_disk_use(true).build();
    let mut final_cursor = col
        .aggregate(final_pipeline)
        .with_options(final_opts)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(doc) = final_cursor
        .try_next()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        && let Ok(br_doc) = doc.get_document("battle_results")
        && let JsonValue::Object(obj) = doc_to_json(br_doc)
    {
        for (key, value) in obj {
            if !key.starts_with("enemy_") {
                combined_results.insert(key, value);
            }
        }
    }

    let totals_pipeline = vec![
        doc! { "$match": base_filter.clone() },
        doc! { "$group": {
            "_id": null,
            "enemy_power": { "$sum": { "$ifNull": [ "$report.battle_results.enemy_power", 0 ] } },
            "enemy_init_max": { "$sum": { "$ifNull": [ "$report.battle_results.enemy_init_max", 0 ] } },
            "enemy_max": { "$sum": { "$ifNull": [ "$report.battle_results.enemy_max", 0 ] } },
            "enemy_healing": { "$sum": { "$ifNull": [ "$report.battle_results.enemy_healing", 0 ] } },
            "enemy_death": { "$sum": { "$ifNull": [ "$report.battle_results.enemy_death", 0 ] } },
            "enemy_severely_wounded": { "$sum": { "$ifNull": [ "$report.battle_results.enemy_severely_wounded", 0 ] } },
            "enemy_wounded": { "$sum": { "$ifNull": [ "$report.battle_results.enemy_wounded", 0 ] } },
            "enemy_remaining": { "$sum": { "$ifNull": [ "$report.battle_results.enemy_remaining", 0 ] } },
            "enemy_watchtower": { "$sum": { "$ifNull": [ "$report.battle_results.enemy_watchtower", 0 ] } },
            "enemy_watchtower_max": { "$sum": { "$ifNull": [ "$report.battle_results.enemy_watchtower_max", 0 ] } },
            "enemy_kill_score": { "$sum": { "$ifNull": [ "$report.battle_results.enemy_kill_score", 0 ] } },
        }},
        doc! { "$project": {
            "_id": 0,
            "battle_results": {
                "enemy_power": "$enemy_power",
                "enemy_init_max": "$enemy_init_max",
                "enemy_max": "$enemy_max",
                "enemy_healing": "$enemy_healing",
                "enemy_death": "$enemy_death",
                "enemy_severely_wounded": "$enemy_severely_wounded",
                "enemy_wounded": "$enemy_wounded",
                "enemy_remaining": "$enemy_remaining",
                "enemy_watchtower": "$enemy_watchtower",
                "enemy_watchtower_max": "$enemy_watchtower_max",
                "enemy_kill_score": "$enemy_kill_score",
            }
        }},
    ];

    let totals_opts = AggregateOptions::builder().allow_disk_use(true).build();
    let mut totals_cursor = col
        .aggregate(totals_pipeline)
        .with_options(totals_opts)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(doc) = totals_cursor
        .try_next()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        && let Ok(br_doc) = doc.get_document("battle_results")
        && let JsonValue::Object(obj) = doc_to_json(br_doc)
    {
        for (key, value) in obj {
            combined_results.insert(key, value);
        }
    }

    let report_summary = if combined_results.is_empty() {
        None
    } else {
        Some(ApiReportSummary {
            battle_results: Some(JsonValue::Object(combined_results)),
        })
    };

    Ok(Json(ApiDetailResponse {
        parent_hash,
        items,
        next_cursor,
        count,
        report: report_summary,
    }))
}
