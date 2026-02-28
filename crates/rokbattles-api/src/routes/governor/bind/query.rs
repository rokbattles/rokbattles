use crate::error::ApiError;
use crate::routes::governor::common::parse_positive_governor_id_str;

/// Parse `governorId` from path parameters.
pub(super) fn parse_governor_id(raw_governor_id: &str) -> Result<i64, ApiError> {
    parse_positive_governor_id_str(raw_governor_id)
        .ok_or_else(|| ApiError::bad_request("Invalid governorId"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_governor_id_accepts_positive_integer() {
        assert_eq!(parse_governor_id("123").expect("governor id"), 123);
        assert_eq!(parse_governor_id(" 456 ").expect("governor id"), 456);
    }

    #[test]
    fn parse_governor_id_rejects_invalid_values() {
        assert!(parse_governor_id("").is_err());
        assert!(parse_governor_id("0").is_err());
        assert!(parse_governor_id("-1").is_err());
        assert!(parse_governor_id("abc").is_err());
    }
}
