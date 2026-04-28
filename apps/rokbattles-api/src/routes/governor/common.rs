use std::collections::HashMap;

use mongodb::bson::doc;

use crate::{
    error::ApiError,
    routes::governor::date_range::{GovernorDateRange, parse_governor_date_range},
    state::AppState,
};

const DEFAULT_MAX_RANGE_DAYS: i64 = 366;

fn parse_positive_governor_id_str(value: &str) -> Option<i64> {
    let parsed = value.trim().parse::<i64>().ok()?;
    (parsed > 0).then_some(parsed)
}

/// Parse a governor id from a path parameter.
pub(crate) fn parse_governor_id_param(raw_governor_id: &str) -> Result<i64, ApiError> {
    parse_positive_governor_id_str(raw_governor_id)
        .ok_or_else(|| ApiError::bad_request("Invalid governorId"))
}

/// Parse the standard `start` / `end` governor date-range query with the default cap.
pub(crate) fn parse_default_governor_date_range(
    params: &HashMap<String, String>,
) -> Result<GovernorDateRange, ApiError> {
    parse_governor_date_range(params, DEFAULT_MAX_RANGE_DAYS)
}

pub(crate) async fn ensure_governor_claim_for_user(
    state: &AppState,
    discord_id: &str,
    governor_id: i64,
) -> Result<(), ApiError> {
    let claim = state
        .reports_store
        .claimed_governors_collection()
        .find_one(doc! {
            "discordId": discord_id,
            "governorId": governor_id
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    if claim.is_none() {
        return Err(ApiError::not_found("Claim not found"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_governor_id_param_accepts_positive_integer() {
        assert_eq!(parse_governor_id_param("123").expect("governor id"), 123);
        assert_eq!(parse_governor_id_param(" 456 ").expect("governor id"), 456);
    }

    #[test]
    fn parse_governor_id_param_rejects_non_positive_or_invalid_values() {
        assert!(parse_governor_id_param("0").is_err());
        assert!(parse_governor_id_param("-1").is_err());
        assert!(parse_governor_id_param("abc").is_err());
    }

    #[test]
    fn parse_default_governor_date_range_reads_start_and_end() {
        let range = parse_default_governor_date_range(&HashMap::from([
            ("start".to_string(), "2025-02-03".to_string()),
            ("end".to_string(), "2025-02-04".to_string()),
        ]))
        .expect("range");

        assert_eq!(range.start, "2025-02-03");
        assert_eq!(range.end, "2025-02-04");
    }
}
