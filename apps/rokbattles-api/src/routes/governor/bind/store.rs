use futures::StreamExt;
use mongodb::{
    Collection,
    bson::{DateTime, Document, doc},
    options::{FindOneOptions, FindOptions},
};

use crate::{bson_utils::bson_to_i64_exact, error::ApiError};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct UserClaimSummary {
    pub claim_count: u64,
    pub has_default_claim: bool,
}

/// Check whether this governor is already claimed by any user.
pub(super) async fn governor_claim_exists(
    claims: &Collection<Document>,
    governor_id: i64,
) -> Result<bool, ApiError> {
    claims
        .find_one(doc! { "governorId": governor_id })
        .await
        .map(|document| document.is_some())
        .map_err(|error| ApiError::internal(error.to_string()))
}

/// Read user claim count plus whether a default bind already exists.
pub(super) async fn summarize_user_claims(
    claims: &Collection<Document>,
    discord_id: &str,
    max_governor_binds: u64,
) -> Result<UserClaimSummary, ApiError> {
    let max_query_docs = i64::try_from(max_governor_binds.saturating_add(1)).unwrap_or(i64::MAX);
    let options = FindOptions::builder()
        .projection(doc! { "_id": 0, "default": 1 })
        .limit(max_query_docs)
        .build();

    let mut cursor = claims
        .find(doc! { "discordId": discord_id })
        .with_options(options)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut summary = UserClaimSummary::default();
    while let Some(next) = cursor.next().await {
        let claim = next.map_err(|error| ApiError::internal(error.to_string()))?;
        summary.claim_count += 1;
        summary.has_default_claim |= claim_document_is_default(&claim);
    }

    Ok(summary)
}

/// Insert one claimed governor bind.
pub(super) async fn insert_claim(
    claims: &Collection<Document>,
    discord_id: &str,
    governor_id: i64,
    governor_name: Option<&str>,
    governor_avatar: Option<&str>,
    default: bool,
) -> Result<(), ApiError> {
    claims
        .insert_one(doc! {
            "discordId": discord_id,
            "governorId": governor_id,
            "createdAt": DateTime::now(),
            "governorName": governor_name,
            "governorAvatar": governor_avatar,
            "default": default,
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok(())
}

/// Check if the user has already claimed this governor.
pub(super) async fn claim_exists_for_user(
    claims: &Collection<Document>,
    discord_id: &str,
    governor_id: i64,
) -> Result<bool, ApiError> {
    claims
        .find_one(doc! { "discordId": discord_id, "governorId": governor_id })
        .await
        .map(|document| document.is_some())
        .map_err(|error| ApiError::internal(error.to_string()))
}

/// Delete one claimed governor for the user and return the deleted document.
pub(super) async fn delete_claim_for_user(
    claims: &Collection<Document>,
    discord_id: &str,
    governor_id: i64,
) -> Result<Option<Document>, ApiError> {
    claims
        .find_one_and_delete(doc! { "discordId": discord_id, "governorId": governor_id })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))
}

/// Returns whether a claim document is marked as default.
pub(super) fn claim_document_is_default(claim: &Document) -> bool {
    claim.get_bool("default").unwrap_or(false)
}

/// Reset previous defaults and set one bind as default.
pub(super) async fn set_bind_as_default(
    claims: &Collection<Document>,
    discord_id: &str,
    governor_id: i64,
) -> Result<(), ApiError> {
    claims
        .update_many(
            doc! { "discordId": discord_id, "default": true },
            doc! { "$set": { "default": false } },
        )
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    claims
        .update_one(
            doc! { "discordId": discord_id, "governorId": governor_id },
            doc! { "$set": { "default": true } },
        )
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok(())
}

/// Find the most recently created claim for the user.
pub(super) async fn find_most_recent_governor_id(
    claims: &Collection<Document>,
    discord_id: &str,
) -> Result<Option<i64>, ApiError> {
    let most_recent = claims
        .find_one(doc! { "discordId": discord_id })
        .with_options(
            FindOneOptions::builder()
                .sort(doc! { "createdAt": -1 })
                .projection(doc! { "governorId": 1 })
                .build(),
        )
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok(most_recent.as_ref().and_then(extract_governor_id_from_claim))
}

fn extract_governor_id_from_claim(claim: &Document) -> Option<i64> {
    claim.get("governorId").and_then(bson_to_i64_exact)
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{Bson, doc};

    use super::{claim_document_is_default, extract_governor_id_from_claim};

    #[test]
    fn reads_default_flag_from_claim_document() {
        let with_default = doc! { "default": true };
        let without_default = doc! { "governorId": 10 };

        assert!(claim_document_is_default(&with_default));
        assert!(!claim_document_is_default(&without_default));
    }

    #[test]
    fn extracts_governor_id_from_supported_bson_number_types() {
        assert_eq!(
            extract_governor_id_from_claim(&doc! { "governorId": Bson::Int32(12) }),
            Some(12)
        );
        assert_eq!(
            extract_governor_id_from_claim(&doc! { "governorId": Bson::Int64(34) }),
            Some(34)
        );
        assert_eq!(
            extract_governor_id_from_claim(&doc! { "governorId": Bson::Double(56.0) }),
            Some(56)
        );
        assert_eq!(
            extract_governor_id_from_claim(&doc! { "governorId": Bson::Double(56.1) }),
            None
        );
        assert_eq!(extract_governor_id_from_claim(&doc! { "governorId": Bson::Null }), None);
    }
}
