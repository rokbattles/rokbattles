use std::collections::HashMap;

use crate::error::ApiError;

const DEFAULT_LIMIT: i64 = 100;
const MAX_LIMIT: i64 = 250;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ArkListRequest {
    pub limit: i64,
}

pub(crate) fn parse_ark_list_request(
    params: &HashMap<String, String>,
) -> Result<ArkListRequest, ApiError> {
    let limit = resolve_limit(params.get("limit").map(String::as_str));
    Ok(ArkListRequest { limit })
}

pub(crate) fn parse_match_id(raw_match_id: &str) -> Result<String, ApiError> {
    let normalized = raw_match_id.trim();
    if normalized.is_empty() {
        return Err(ApiError::bad_request("Invalid match id"));
    }

    Ok(normalized.to_string())
}

fn resolve_limit(value: Option<&str>) -> i64 {
    let parsed =
        value.map(str::trim).filter(|raw| !raw.is_empty()).and_then(|raw| raw.parse::<f64>().ok());

    let Some(parsed) = parsed else {
        return DEFAULT_LIMIT;
    };

    if !parsed.is_finite() {
        return DEFAULT_LIMIT;
    }

    (parsed.floor() as i64).clamp(1, MAX_LIMIT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_list_request_uses_default_limit_for_missing_or_invalid_value() {
        let default_request = parse_ark_list_request(&HashMap::new()).expect("request");
        assert_eq!(default_request.limit, DEFAULT_LIMIT);

        let invalid_request =
            parse_ark_list_request(&HashMap::from([("limit".to_string(), "abc".to_string())]))
                .expect("request");
        assert_eq!(invalid_request.limit, DEFAULT_LIMIT);
    }

    #[test]
    fn parse_list_request_clamps_limit_to_safe_bounds() {
        let min_request =
            parse_ark_list_request(&HashMap::from([("limit".to_string(), "-10".to_string())]))
                .expect("request");
        assert_eq!(min_request.limit, 1);

        let max_request =
            parse_ark_list_request(&HashMap::from([("limit".to_string(), "999".to_string())]))
                .expect("request");
        assert_eq!(max_request.limit, MAX_LIMIT);
    }

    #[test]
    fn parse_match_id_trims_and_rejects_empty_values() {
        assert_eq!(parse_match_id("  mail-123  ").expect("match id"), "mail-123");
        assert!(parse_match_id("   ").is_err());
    }
}
