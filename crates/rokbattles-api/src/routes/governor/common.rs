use mongodb::bson::doc;

use crate::error::ApiError;
use crate::state::AppState;

pub(crate) fn parse_positive_governor_id_str(value: &str) -> Option<i64> {
    let parsed = value.trim().parse::<i64>().ok()?;
    (parsed > 0).then_some(parsed)
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
    fn parses_positive_governor_id_from_string() {
        assert_eq!(parse_positive_governor_id_str("123"), Some(123));
        assert_eq!(parse_positive_governor_id_str(" 456 "), Some(456));
    }

    #[test]
    fn rejects_non_positive_or_invalid_governor_ids() {
        assert_eq!(parse_positive_governor_id_str("0"), None);
        assert_eq!(parse_positive_governor_id_str("-1"), None);
        assert_eq!(parse_positive_governor_id_str("abc"), None);
    }
}
