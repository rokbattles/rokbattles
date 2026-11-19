//! Resource transfers for an account's claimed governors.

use std::{num::NonZeroUsize, sync::Arc};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::header,
    response::IntoResponse,
};
use rustc_hash::FxHashMap;
use serde::Deserialize;

use crate::{
    error::ApiError,
    routes::{
        governor::{
            common::{
                ensure_governor_claim_for_user, parse_default_governor_date_range,
                parse_governor_id_param,
            },
            date_range::GovernorDateRange,
        },
        me::AccountAccess,
    },
    state::AppState,
};

mod aggregate;
mod store;

#[derive(Deserialize)]
pub(crate) struct CarriageQuery {
    page: Option<NonZeroUsize>,
    start: Option<String>,
    end: Option<String>,
}

impl CarriageQuery {
    fn date_range(&self) -> Result<Option<GovernorDateRange>, ApiError> {
        match (&self.start, &self.end) {
            (None, None) => Ok(None),
            (Some(start), Some(end)) => {
                let range = parse_default_governor_date_range(&FxHashMap::from_iter([
                    ("start".to_owned(), start.clone()),
                    ("end".to_owned(), end.clone()),
                ]))?;

                // The shared parser can fall back to the current year or cap
                // the end date. Require the exact range the caller requested.
                if range.start != *start || range.end != *end {
                    return Err(ApiError::bad_request("Invalid transfer date range"));
                }

                Ok(Some(range))
            }
            _ => Err(ApiError::bad_request("Both start and end dates are required")),
        }
    }
}

/// Returns resource totals, top counterparties, and one page of transfer groups.
///
/// The signed-in account must have claimed the governor. Dates are inclusive
/// UTC days in `YYYY-MM-DD` format. Omit both dates to include all reports.
pub(crate) async fn get(
    State(state): State<Arc<AppState>>,
    Path(governor_id): Path<String>,
    Query(query): Query<CarriageQuery>,
    access: AccountAccess,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id)?;
    ensure_governor_claim_for_user(&state, access.account_id(), governor_id).await?;

    let range = query.date_range()?;
    let reports = store::fetch_reports(&state, governor_id, range.as_ref()).await?;
    let response = aggregate::summarize(reports, query.page.unwrap_or(NonZeroUsize::MIN));

    Ok(([(header::CACHE_CONTROL, "no-store")], Json(response)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_date_filter_accepts_all_time_and_includes_the_end_day() {
        let query = CarriageQuery { page: None, start: None, end: None };
        assert!(query.date_range().expect("valid query").is_none());

        let query = CarriageQuery {
            page: None,
            start: Some("2026-09-10".to_owned()),
            end: Some("2026-09-10".to_owned()),
        };
        let range = query.date_range().expect("valid query").expect("date range");
        assert_eq!((range.start_millis, range.end_millis), (1_788_998_400_000, 1_789_084_800_000));
    }

    #[test]
    fn transfer_date_filter_rejects_invalid_or_partial_ranges() {
        for (start, end) in [
            (Some("2026-09-12"), Some("2026-09-10")),
            (Some("2026-02-30"), Some("2026-03-01")),
            (Some("2026-09-10"), None),
            (Some("2025-01-01"), Some("2026-09-10")),
        ] {
            let query = CarriageQuery {
                page: None,
                start: start.map(str::to_owned),
                end: end.map(str::to_owned),
            };
            query.date_range().expect_err("invalid date range");
        }
    }
}
