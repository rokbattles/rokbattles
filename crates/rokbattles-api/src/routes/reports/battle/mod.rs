use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::{Json, http::StatusCode};
use futures::StreamExt;
use mongodb::bson::doc;
use mongodb::options::{FindOneOptions, FindOptions};

use crate::error::ApiError;
use crate::state::AppState;

use self::detail_mapper::{build_report_detail_filter, report_detail_projection};
use self::mapper::{map_report_document, reports_projection};
use self::match_builder::build_reports_match;
use self::query::parse_reports_request;
use self::types::{ReportByIdResponse, ReportRowWithCursor, ReportsResponse};

mod detail_mapper;
mod mapper;
mod match_builder;
mod query;
mod types;

const PAGE_SIZE: usize = 100;
const FETCH_LIMIT: i64 = PAGE_SIZE as i64 + 1;

/// Lists battle reports with filters and cursor pagination.
pub async fn get(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let request = parse_reports_request(&params)?;
    let final_match = build_reports_match(&request);

    let options = FindOptions::builder()
        .sort(doc! { "metadata.mail_time": request.sort_direction() })
        .limit(FETCH_LIMIT)
        .projection(reports_projection())
        .build();

    let mut cursor = state
        .reports_store
        .battle_collection()
        .find(final_match)
        .with_options(options)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut fetched_documents = 0usize;
    let mut rows: Vec<ReportRowWithCursor> = Vec::new();
    while let Some(next) = cursor.next().await {
        fetched_documents += 1;
        let document = next.map_err(|error| ApiError::internal(error.to_string()))?;
        if let Some(row) = map_report_document(&document) {
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
            Some(first_row.mail_time.to_string())
        } else {
            None
        }
    } else {
        None
    };

    let next_after = if let Some(last_row) = last_row {
        if request.before_cursor.is_some() || has_more_in_query_direction {
            Some(last_row.mail_time.to_string())
        } else {
            None
        }
    } else {
        None
    };

    let response = ReportsResponse {
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

/// Looks up a single battle report by mail id.
pub async fn get_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let report_id = parse_report_id(&id)?;

    let options = FindOneOptions::builder()
        .projection(report_detail_projection())
        .build();

    let mail = state
        .reports_store
        .battle_collection()
        .find_one(build_report_detail_filter(&report_id))
        .with_options(options)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let response = ReportByIdResponse {
        id: report_id,
        mail,
    };

    Ok((
        StatusCode::OK,
        [("Cache-Control", "no-store")],
        Json(response),
    ))
}

fn parse_report_id(raw_id: &str) -> Result<String, ApiError> {
    let normalized = raw_id.trim();
    if normalized.is_empty() {
        return Err(ApiError::bad_request("Invalid report id"));
    }

    Ok(normalized.to_string())
}

#[cfg(test)]
mod tests {
    use super::parse_report_id;

    #[test]
    fn parses_non_empty_report_id() {
        let parsed = parse_report_id("mail-123").expect("id should parse");
        assert_eq!(parsed, "mail-123");
    }

    #[test]
    fn trims_and_rejects_empty_report_id() {
        assert_eq!(
            parse_report_id("  mail-123  ").expect("id should parse"),
            "mail-123"
        );
        assert!(parse_report_id("   ").is_err());
    }
}
