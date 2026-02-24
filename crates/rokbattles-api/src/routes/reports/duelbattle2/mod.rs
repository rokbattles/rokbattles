use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::{Json, http::StatusCode};
use futures::StreamExt;
use mongodb::options::AggregateOptions;

use crate::error::ApiError;
use crate::state::AppState;

use self::mapper::{build_duelbattle2_pipeline, map_duelbattle2_document};
use self::query::parse_duelbattle2_request;
use self::types::{DuelBattle2Response, DuelBattle2RowWithCursor};

mod mapper;
mod query;
mod types;

const PAGE_SIZE: usize = 100;
const FETCH_LIMIT: i64 = PAGE_SIZE as i64 + 1;

/// Olympian Arena duel reports listing endpoint with cursor pagination.
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
