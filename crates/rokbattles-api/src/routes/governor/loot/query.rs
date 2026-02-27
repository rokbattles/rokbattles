use std::collections::HashMap;

use mongodb::bson::{DateTime, Document, doc};

use crate::error::ApiError;
use crate::routes::governor::common::parse_positive_governor_id_str;
use crate::time_utils::date_key_utc;

const DEFAULT_MAX_RANGE_DAYS: i64 = 366;
const ONE_DAY_MILLIS: i64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Clone)]
pub(crate) struct LootRequest {
    pub range: LootDateRange,
}

#[derive(Debug, Clone)]
pub(crate) struct LootDateRange {
    pub start_millis: i64,
    pub end_millis: i64,
    pub start: String,
    pub end: String,
}

impl LootDateRange {
    pub fn build_mail_time_match(&self) -> Document {
        let start_seconds = self.start_millis / 1000;
        let end_seconds = self.end_millis / 1000;
        let start_micros = self.start_millis.saturating_mul(1000);
        let end_micros = self.end_millis.saturating_mul(1000);

        doc! {
            "$or": [
                { "metadata.mail_time": { "$gte": start_seconds, "$lt": end_seconds } },
                { "metadata.mail_time": { "$gte": self.start_millis, "$lt": self.end_millis } },
                { "metadata.mail_time": { "$gte": start_micros, "$lt": end_micros } },
            ]
        }
    }
}

pub(crate) fn parse_governor_id(raw_governor_id: &str) -> Result<i64, ApiError> {
    parse_positive_governor_id_str(raw_governor_id)
        .ok_or_else(|| ApiError::bad_request("Invalid governorId"))
}

pub(crate) fn parse_loot_request(
    params: &HashMap<String, String>,
) -> Result<LootRequest, ApiError> {
    let fallback_year = current_utc_year()?;
    let range = resolve_date_range(
        params.get("start").map(String::as_str),
        params.get("end").map(String::as_str),
        fallback_year,
        DEFAULT_MAX_RANGE_DAYS,
    )?;

    Ok(LootRequest { range })
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
) -> Result<LootDateRange, ApiError> {
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

    let max_range_millis = max_range_days * ONE_DAY_MILLIS;
    let capped_end = end_millis.min(start_millis + max_range_millis);
    let final_end_millis = capped_end.max(start_millis + ONE_DAY_MILLIS);

    let start = date_key_utc(start_millis).ok_or_else(|| ApiError::bad_request("Invalid start"))?;
    let end =
        date_key_utc(final_end_millis - 1).ok_or_else(|| ApiError::bad_request("Invalid end"))?;

    Ok(LootDateRange {
        start_millis,
        end_millis: final_end_millis,
        start,
        end,
    })
}

fn parse_date_start_millis(value: &str) -> Option<i64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let rfc3339 = format!("{value}T00:00:00Z");
    DateTime::parse_rfc3339_str(rfc3339)
        .ok()
        .map(DateTime::timestamp_millis)
}

fn parse_date_end_inclusive_millis(value: &str) -> Option<i64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let rfc3339 = format!("{value}T23:59:59.999Z");
    DateTime::parse_rfc3339_str(rfc3339)
        .ok()
        .map(DateTime::timestamp_millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_governor_id_accepts_positive_integer() {
        assert_eq!(parse_governor_id("123").expect("governor id"), 123);
        assert_eq!(parse_governor_id(" 42 ").expect("governor id"), 42);
    }

    #[test]
    fn parse_governor_id_rejects_invalid_values() {
        assert!(parse_governor_id("").is_err());
        assert!(parse_governor_id("-1").is_err());
        assert!(parse_governor_id("abc").is_err());
    }

    #[test]
    fn resolve_date_range_uses_custom_start_and_end_when_valid() {
        let range = resolve_date_range(
            Some("2025-02-03"),
            Some("2025-02-04"),
            2024,
            DEFAULT_MAX_RANGE_DAYS,
        )
        .expect("range");
        assert_eq!(range.start, "2025-02-03");
        assert_eq!(range.end, "2025-02-04");
    }

    #[test]
    fn resolve_date_range_falls_back_to_year_on_invalid_window() {
        let range = resolve_date_range(
            Some("2025-02-05"),
            Some("2025-02-04"),
            2024,
            DEFAULT_MAX_RANGE_DAYS,
        )
        .expect("range");
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
