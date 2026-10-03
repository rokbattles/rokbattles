//! Header rules shared by the direct and relay upload endpoints.

use axum::http::HeaderMap;

use crate::error::ApiError;

pub(super) fn is_allowed_content_type(content_type: &str) -> bool {
    content_type.eq_ignore_ascii_case("application/octet-stream")
}

/// Reads and validates the user-agent header.
///
/// Accepts `ROKBattles/<version>` with a nonempty version and an optional suffix:
/// `(Relay)`, or text starting with `(` and containing ` Tauri/`. For example,
/// `ROKBattles/1.6.1 (Relay)` and `ROKBattles/0.2.5 (MacOS; Tauri/1.5.0)` are accepted.
/// This checks formatting, not client identity or semantic-version validity.
pub(super) fn extract_user_agent(headers: &HeaderMap) -> Result<&str, ApiError> {
    let user_agent = headers
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ApiError::bad_request("missing user-agent"))?;

    if !is_valid_user_agent(user_agent) {
        return Err(ApiError::bad_request("bad user agent"));
    }

    Ok(user_agent)
}

fn is_valid_user_agent(user_agent: &str) -> bool {
    let Some(rest) = user_agent.strip_prefix("ROKBattles/") else {
        return false;
    };

    let mut parts = rest.splitn(2, ' ');
    let version = parts.next().unwrap_or_default();
    if version.is_empty() {
        return false;
    }

    match parts.next() {
        None => true,
        Some("(Relay)") => true,
        Some(remainder) => remainder.starts_with('(') && remainder.contains(" Tauri/"),
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn validates_content_type() {
        assert!(is_allowed_content_type("application/octet-stream"));
        assert!(!is_allowed_content_type("application/json"));
        assert!(!is_allowed_content_type("text/plain"));
    }

    #[test]
    fn extract_user_agent_accepts_valid_header() {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("ROKBattles/0.1.0"));
        let user_agent = extract_user_agent(&headers).unwrap();
        assert_eq!(user_agent, "ROKBattles/0.1.0");
    }

    #[test]
    fn extract_user_agent_rejects_missing_header() {
        let headers = HeaderMap::new();
        let err = extract_user_agent(&headers).unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn extract_user_agent_rejects_invalid_header() {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("OtherApp/0.1.0"));
        let err = extract_user_agent(&headers).unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn is_valid_user_agent_accepts_minimal_user_agent() {
        assert!(is_valid_user_agent("ROKBattles/0.1.0"));
    }

    #[test]
    fn is_valid_user_agent_accepts_tauri_suffix() {
        assert!(is_valid_user_agent("ROKBattles/0.2.5 (MacOS; Tauri/1.5.0)"));
    }

    #[test]
    fn is_valid_user_agent_accepts_relay_suffix() {
        assert!(is_valid_user_agent("ROKBattles/1.6.1 (Relay)"));
    }

    #[test]
    fn is_valid_user_agent_rejects_missing_prefix() {
        assert!(!is_valid_user_agent("OtherApp/0.1.0"));
    }

    #[test]
    fn is_valid_user_agent_rejects_missing_version() {
        assert!(!is_valid_user_agent("ROKBattles/"));
    }

    #[test]
    fn is_valid_user_agent_rejects_suffix_without_tauri() {
        assert!(!is_valid_user_agent("ROKBattles/0.1.0 (MacOS; SomethingElse/1.2.3)"));
    }
}
