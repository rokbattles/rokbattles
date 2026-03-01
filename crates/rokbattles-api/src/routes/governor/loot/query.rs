use std::collections::HashMap;

use crate::error::ApiError;
use crate::routes::governor::common::parse_positive_governor_id_str;
use crate::routes::governor::date_range::{GovernorDateRange, parse_governor_date_range};

const DEFAULT_MAX_RANGE_DAYS: i64 = 366;

#[derive(Debug, Clone)]
pub(crate) struct LootRequest {
    pub range: GovernorDateRange,
}

pub(crate) fn parse_governor_id(raw_governor_id: &str) -> Result<i64, ApiError> {
    parse_positive_governor_id_str(raw_governor_id)
        .ok_or_else(|| ApiError::bad_request("Invalid governorId"))
}

pub(crate) fn parse_loot_request(
    params: &HashMap<String, String>,
) -> Result<LootRequest, ApiError> {
    let range = parse_governor_date_range(params, DEFAULT_MAX_RANGE_DAYS)?;

    Ok(LootRequest { range })
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
    fn parse_loot_request_uses_governor_date_range() {
        let request = parse_loot_request(&HashMap::from([
            ("start".to_string(), "2025-02-03".to_string()),
            ("end".to_string(), "2025-02-04".to_string()),
        ]))
        .expect("request");
        assert_eq!(request.range.start, "2025-02-03");
        assert_eq!(request.range.end, "2025-02-04");
    }
}
