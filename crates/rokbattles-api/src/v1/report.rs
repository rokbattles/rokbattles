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
    report: serde_json::Value,
    start_date: i64,
}

#[derive(Debug, Serialize)]
pub struct ApiDetailResponse {
    parent_hash: String,
    items: Vec<ApiReportEntry>,
    next_cursor: Option<String>,
    count: usize,
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

fn doc_to_json(doc: &Document) -> serde_json::Value {
    bson::from_bson::<serde_json::Value>(Bson::Document(doc.clone()))
        .unwrap_or(serde_json::Value::Null)
}

pub async fn report_by_parent(
    State(st): State<AppState>,
    Path(parent_hash): Path<String>,
    Query(params): Query<DetailParams>,
) -> Result<Json<ApiDetailResponse>, StatusCode> {
    let col = st.db.collection::<Document>("battleReports");

    let limit = params.limit.unwrap_or(50).clamp(1, 200) as i64;
    let (cursor_ts, cursor_hash) = parse_cursor(params.after.as_deref());

    let mut pipeline = vec![
        doc! { "$match": { "metadata.parentHash": &parent_hash }},
        doc! { "$project": {
            "hash": "$metadata.hash",
            "report": "$report",
            "startDate": "$report.metadata.start_date",
        }},
        doc! { "$sort": { "startDate": -1_i32, "hash": -1_i32 }},
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
            _ => serde_json::Value::Null,
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
    Ok(Json(ApiDetailResponse {
        parent_hash,
        items,
        next_cursor,
        count,
    }))
}
