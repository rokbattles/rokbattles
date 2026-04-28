use std::collections::HashMap;

use mongodb::bson::{DateTime, Document};

use crate::{
    error::ApiError,
    time_utils::{build_mail_time_match, date_key_utc},
};

const ONE_DAY_MILLIS: i64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Clone)]
pub(crate) struct GovernorDateRange {
    pub start_millis: i64,
    pub end_millis: i64,
    pub start: String,
    pub end: String,
}

impl GovernorDateRange {
    pub fn build_mail_time_match(&self) -> Document {
        build_mail_time_match(self.start_millis, self.end_millis)
    }
}

pub(crate) fn parse_governor_date_range(
    params: &HashMap<String, String>,
    max_range_days: i64,
) -> Result<GovernorDateRange, ApiError> {
    let fallback_year = current_utc_year()?;
    resolve_date_range(
        params.get("start").map(String::as_str),
        params.get("end").map(String::as_str),
        fallback_year,
        max_range_days,
    )
}

fn current_utc_year() -> Result<i32, ApiError> {
    DateTime::now()
        .try_to_rfc3339_string()
        .ok()
        .and_then(|value| value.get(0..4).and_then(|year| year.parse::<i32>().ok()))
        .ok_or_else(|| ApiError::internal("Failed to resolve current UTC year"))
}

fn resolve_date_range(
    start_param: Option<&str>,
    end_param: Option<&str>,
    fallback_year: i32,
    max_range_days: i64,
) -> Result<GovernorDateRange, ApiError> {
    let parsed_start = start_param.and_then(parse_date_start_millis);
    let parsed_end_inclusive = end_param.and_then(parse_date_end_inclusive_millis);

    let default_start = parse_date_start_millis(&format!("{fallback_year:04}-01-01"))
        .ok_or_else(|| ApiError::bad_request("Invalid year range"))?;
    let default_end = parse_date_start_millis(&format!("{:04}-01-01", fallback_year + 1))
        .ok_or_else(|| ApiError::bad_request("Invalid year range"))?;

    let (start_millis, end_millis) = match (parsed_start, parsed_end_inclusive) {
        (Some(start), Some(end_inclusive)) if end_inclusive + 1 > start => {
            (start, end_inclusive + 1)
        }
        _ => (default_start, default_end),
    };

    let max_range_millis = max_range_days.saturating_mul(ONE_DAY_MILLIS);
    let capped_end = end_millis.min(start_millis.saturating_add(max_range_millis));
    let final_end_millis = capped_end.max(start_millis.saturating_add(ONE_DAY_MILLIS));

    let start = date_key_utc(start_millis).ok_or_else(|| ApiError::bad_request("Invalid start"))?;
    let end =
        date_key_utc(final_end_millis - 1).ok_or_else(|| ApiError::bad_request("Invalid end"))?;

    Ok(GovernorDateRange { start_millis, end_millis: final_end_millis, start, end })
}

fn parse_date_start_millis(value: &str) -> Option<i64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let rfc3339 = format!("{value}T00:00:00Z");
    DateTime::parse_rfc3339_str(rfc3339).ok().map(DateTime::timestamp_millis)
}

fn parse_date_end_inclusive_millis(value: &str) -> Option<i64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let rfc3339 = format!("{value}T23:59:59.999Z");
    DateTime::parse_rfc3339_str(rfc3339).ok().map(DateTime::timestamp_millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_date_range_uses_custom_start_and_end_when_valid() {
        let range =
            resolve_date_range(Some("2025-02-03"), Some("2025-02-04"), 2024, 366).expect("range");
        assert_eq!(range.start, "2025-02-03");
        assert_eq!(range.end, "2025-02-04");
    }

    #[test]
    fn resolve_date_range_falls_back_to_year_on_invalid_window() {
        let range =
            resolve_date_range(Some("2025-02-05"), Some("2025-02-04"), 2024, 366).expect("range");
        assert_eq!(range.start, "2024-01-01");
        assert_eq!(range.end, "2024-12-31");
    }

    #[test]
    fn resolve_date_range_caps_end_to_max_days() {
        let range =
            resolve_date_range(Some("2025-01-01"), Some("2027-01-01"), 2024, 10).expect("range");
        assert_eq!(range.start, "2025-01-01");
        assert_eq!(range.end, "2025-01-10");
    }
}
