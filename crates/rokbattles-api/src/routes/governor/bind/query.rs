use std::collections::HashMap;

use axum::body::Bytes;
use serde_json::Value;

use crate::error::ApiError;

use super::super::common::{
    parse_positive_governor_id_from_json, parse_positive_governor_id_from_query,
};

/// Parse `governorId` from the bind request body.
pub(super) fn parse_governor_id_from_bind_body(body: &Bytes) -> Result<i64, ApiError> {
    let payload: Value =
        serde_json::from_slice(body).map_err(|_| ApiError::bad_request("Invalid JSON body"))?;

    parse_positive_governor_id_from_json(payload.get("governorId"))
        .ok_or_else(|| ApiError::bad_request("Invalid governorId"))
}

/// Parse `governorId` from query parameters.
pub(super) fn parse_governor_id_from_query_params(
    params: &HashMap<String, String>,
) -> Result<i64, ApiError> {
    parse_positive_governor_id_from_query(params)
        .ok_or_else(|| ApiError::bad_request("Invalid governorId"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_governor_id_from_bind_body_number() {
        let payload = Bytes::from_static(br#"{"governorId":123}"#);
        assert_eq!(
            parse_governor_id_from_bind_body(&payload).expect("governor id"),
            123
        );
    }

    #[test]
    fn parses_governor_id_from_bind_body_string() {
        let payload = Bytes::from_static(br#"{"governorId":"456"}"#);
        assert_eq!(
            parse_governor_id_from_bind_body(&payload).expect("governor id"),
            456
        );
    }

    #[test]
    fn rejects_invalid_bind_body_payloads() {
        let invalid_json = Bytes::from_static(br#"{"governorId":}"#);
        let missing_governor_id = Bytes::from_static(br#"{"foo":"bar"}"#);
        let zero_governor_id = Bytes::from_static(br#"{"governorId":0}"#);

        assert!(parse_governor_id_from_bind_body(&invalid_json).is_err());
        assert!(parse_governor_id_from_bind_body(&missing_governor_id).is_err());
        assert!(parse_governor_id_from_bind_body(&zero_governor_id).is_err());
    }

    #[test]
    fn parses_governor_id_from_query_params() {
        let params = HashMap::from([("governorId".to_string(), "789".to_string())]);
        assert_eq!(
            parse_governor_id_from_query_params(&params).expect("governor id"),
            789
        );
    }
}
