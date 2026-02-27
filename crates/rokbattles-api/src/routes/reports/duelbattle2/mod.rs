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
use crate::state::AppState;

use self::detail_mapper::{
    build_duelbattle2_detail_filter, duelbattle2_detail_projection, map_duelbattle2_detail_document,
};
use self::list_mapper::{build_duelbattle2_pipeline, map_duelbattle2_document};
use self::query::parse_duelbattle2_request;
use self::types::{DuelBattle2DetailResponse, DuelBattle2Response, DuelBattle2RowWithCursor};

mod bson_utils;
mod detail_mapper;
mod list_mapper;
mod query;
mod types;

const PAGE_SIZE: usize = 100;
const FETCH_LIMIT: i64 = PAGE_SIZE as i64 + 1;

/// Returns a paginated list of Olympian Arena duels.
pub async fn get(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let request = parse_duelbattle2_request(&params)?;
    let pipeline = build_duelbattle2_pipeline(&request, FETCH_LIMIT);
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
        if let Some(row) = map_duelbattle2_document(&document) {
            rows.push(row);
        }
    }

    let has_more_in_query_direction = fetched_documents > PAGE_SIZE;
    let paged_rows = if has_more_in_query_direction {
        rows.into_iter().take(PAGE_SIZE).collect::<Vec<_>>()
    } else {
        rows
    };

    let ordered_rows = if request.before_cursor.is_some() {
        paged_rows.into_iter().rev().collect::<Vec<_>>()
    } else {
        paged_rows
    };

    let first_row = ordered_rows.first();
    let last_row = ordered_rows.last();

    let previous_before = if let Some(first_row) = first_row {
        if !request.is_initial_page()
            && (request.after_cursor.is_some()
                || (request.before_cursor.is_some() && has_more_in_query_direction))
        {
            Some(first_row.latest_mail_time.to_string())
        } else {
            None
        }
    } else {
        None
    };

    let next_after = if let Some(last_row) = last_row {
        if request.before_cursor.is_some() || has_more_in_query_direction {
            Some(last_row.latest_mail_time.to_string())
        } else {
            None
        }
    } else {
        None
    };

    let response = DuelBattle2Response {
        items: ordered_rows.into_iter().map(|row| row.item).collect(),
        next_after,
        previous_before,
    };

    Ok((
        StatusCode::OK,
        [("Cache-Control", "no-store")],
        Json(response),
    ))
}

/// Returns all report entries for one Olympian Arena duel team.
pub async fn get_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let duel_id = parse_duelbattle2_id(&id)?;

    let options = FindOptions::builder()
        .sort(doc! { "metadata.mail_time": 1 })
        .projection(duelbattle2_detail_projection())
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
        [("Cache-Control", "no-store")],
        Json(response),
    ))
}

fn parse_duelbattle2_id(raw_id: &str) -> Result<i64, ApiError> {
    raw_id
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
    fn rejects_non_numeric_duel_id() {
        let parsed = parse_duelbattle2_id("abc");
        assert!(parsed.is_err());
    }
}
