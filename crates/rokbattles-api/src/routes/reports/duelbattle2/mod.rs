use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::{Json, http::StatusCode};
use futures::StreamExt;
use mongodb::bson::doc;
use mongodb::options::AggregateOptions;
use mongodb::options::FindOptions;

use crate::error::ApiError;
use crate::routes::reports::common::pagination::paginate_cursor_rows;
use crate::state::AppState;

use self::detail_mapper::{
    build_duelbattle2_detail_filter, build_duelbattle2_detail_projection,
    map_duelbattle2_detail_document,
};
use self::list_mapper::{build_duelbattle2_list_pipeline, map_duelbattle2_list_document};
use self::query::parse_duelbattle2_request;
use self::types::{DuelBattle2DetailResponse, DuelBattle2Response, DuelBattle2RowWithCursor};

mod detail_mapper;
mod list_mapper;
mod query;
mod types;

const PAGE_SIZE: usize = 100;
const FETCH_LIMIT: i64 = PAGE_SIZE as i64 + 1;
// 1 month
const REPORT_DETAIL_CACHE_CONTROL: &str = "public, max-age=2592000";

/// Return a paginated list of Olympian Arena duels.
pub async fn get(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let request = parse_duelbattle2_request(&params)?;
    let pipeline = build_duelbattle2_list_pipeline(&request, FETCH_LIMIT);
    let aggregate_options = AggregateOptions::builder().allow_disk_use(true).build();

    let mut cursor = state
        .reports_store
        .duelbattle2_collection()
        .aggregate(pipeline)
        .with_options(aggregate_options)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut fetched_documents = 0usize;
    let mut rows: Vec<DuelBattle2RowWithCursor> = Vec::new();
    while let Some(next) = cursor.next().await {
        fetched_documents += 1;
        let document = next.map_err(|error| ApiError::internal(error.to_string()))?;
        if let Some(row) = map_duelbattle2_list_document(&document) {
            rows.push(row);
        }
    }

    let paged_rows = paginate_cursor_rows(
        rows,
        fetched_documents,
        PAGE_SIZE,
        request.before_cursor,
        request.after_cursor,
        |row: &DuelBattle2RowWithCursor| row.latest_mail_time,
    );

    let response = DuelBattle2Response {
        items: paged_rows.items.into_iter().map(|row| row.item).collect(),
        next_after: paged_rows.next_after,
        previous_before: paged_rows.previous_before,
    };

    Ok((
        StatusCode::OK,
        [("Cache-Control", "no-store")],
        Json(response),
    ))
}

/// Return all report entries for one Olympian Arena duel team.
pub async fn get_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let duel_id = parse_duelbattle2_id(&id)?;

    let options = FindOptions::builder()
        .sort(doc! { "metadata.mail_time": 1 })
        .projection(build_duelbattle2_detail_projection())
        .build();

    let mut cursor = state
        .reports_store
        .duelbattle2_collection()
        .find(build_duelbattle2_detail_filter(duel_id))
        .with_options(options)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut items = Vec::new();
    while let Some(next) = cursor.next().await {
        let document = next.map_err(|error| ApiError::internal(error.to_string()))?;
        if let Some(item) = map_duelbattle2_detail_document(&document) {
            items.push(item);
        }
    }

    let response = DuelBattle2DetailResponse { items };

    Ok((
        StatusCode::OK,
        [("Cache-Control", REPORT_DETAIL_CACHE_CONTROL)],
        Json(response),
    ))
}

fn parse_duelbattle2_id(raw_id: &str) -> Result<i64, ApiError> {
    raw_id
        .trim()
        .parse::<i64>()
        .map_err(|_| ApiError::bad_request("Invalid duel id"))
}

#[cfg(test)]
mod tests {
    use super::parse_duelbattle2_id;

    #[test]
    fn parses_numeric_duel_id() {
        let parsed = parse_duelbattle2_id("42").expect("id should parse");
        assert_eq!(parsed, 42);
    }

    #[test]
    fn trims_duel_id_before_parsing() {
        let parsed = parse_duelbattle2_id("  42  ").expect("id should parse");
        assert_eq!(parsed, 42);
    }

    #[test]
    fn rejects_non_numeric_duel_id() {
        let parsed = parse_duelbattle2_id("abc");
        assert!(parsed.is_err());
    }
}
