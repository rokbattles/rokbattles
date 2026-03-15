use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use futures::StreamExt;
use mongodb::{
    bson::doc,
    options::{FindOneOptions, FindOptions},
};

use self::{
    detail_mapper::{
        build_battle_detail_filter, build_battle_detail_projection, map_battle_detail_document,
    },
    list_mapper::{build_report_dedupe_key, map_battle_list_document},
    match_builder::build_reports_match,
    query::parse_reports_request,
    types::{ReportByIdResponse, ReportRowWithCursor, ReportsResponse},
};
use crate::{
    error::ApiError, routes::reports::common::pagination::paginate_cursor_rows, state::AppState,
};

mod detail_mapper;
mod list_mapper;
mod match_builder;
mod query;
mod types;

const PAGE_SIZE: usize = 100;
const FETCH_LIMIT: i64 = PAGE_SIZE as i64 + 1;
// 1 month
const REPORT_DETAIL_CACHE_CONTROL: &str = "public, max-age=2592000";

/// List battle reports with filters and cursor pagination.
pub async fn get(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let request = parse_reports_request(&params)?;
    let final_match = build_reports_match(&request);

    let options = FindOptions::builder()
        .sort(doc! { "metadata.mail_time": request.sort_direction() })
        .projection(self::list_mapper::build_battle_list_projection())
        .build();

    let mut cursor = state
        .reports_store
        .battle_collection()
        .find(final_match)
        .with_options(options)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut dedupe_keys = HashSet::new();
    let mut rows: Vec<ReportRowWithCursor> = Vec::new();
    while let Some(next) = cursor.next().await {
        let document = next.map_err(|error| ApiError::internal(error.to_string()))?;
        let Some(row) = map_battle_list_document(&document) else {
            continue;
        };
        let dedupe_key = build_report_dedupe_key(&document)
            .unwrap_or_else(|| format!("mail:{}", row.item.mail_id));
        if dedupe_keys.insert(dedupe_key) {
            rows.push(row);
            if rows.len() >= FETCH_LIMIT as usize {
                break;
            }
        }
    }

    let paged_rows = paginate_cursor_rows(
        rows,
        dedupe_keys.len(),
        PAGE_SIZE,
        request.before_cursor,
        request.after_cursor,
        |row: &ReportRowWithCursor| row.mail_time,
    );

    let response = ReportsResponse {
        items: paged_rows.items.into_iter().map(|row| row.item).collect(),
        next_after: paged_rows.next_after,
        previous_before: paged_rows.previous_before,
    };

    Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)))
}

/// Look up a single battle report by mail ID.
pub async fn get_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let report_id = parse_report_id(&id)?;

    let options = FindOneOptions::builder().projection(build_battle_detail_projection()).build();

    let mail = state
        .reports_store
        .battle_collection()
        .find_one(build_battle_detail_filter(&report_id))
        .with_options(options)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let response = ReportByIdResponse {
        id: report_id,
        mail: mail.as_ref().and_then(map_battle_detail_document),
    };

    Ok((StatusCode::OK, [("Cache-Control", REPORT_DETAIL_CACHE_CONTROL)], Json(response)))
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
        assert_eq!(parse_report_id("  mail-123  ").expect("id should parse"), "mail-123");
        assert!(parse_report_id("   ").is_err());
    }
}
