use std::collections::HashMap;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::{Json, http::StatusCode};
use mongodb::Collection;
use mongodb::bson::{DateTime, Document, doc};
use mongodb::options::FindOneOptions;
use serde::Serialize;
use serde_json::Value;

use crate::auth::AuthenticatedSession;
use crate::bson_utils::bson_to_i64_exact;
use crate::error::ApiError;
use crate::governor_bindings::snapshot::find_latest_sender_snapshot;
use crate::state::AppState;

use super::common::{parse_positive_governor_id_from_json, parse_positive_governor_id_from_query};

const MAX_GOVERNOR_BINDS: u64 = 3;

#[derive(Debug, Serialize)]
struct BindGovernorResponse {
    claim: ClaimedGovernor,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaimedGovernor {
    governor_id: i64,
    governor_name: Option<String>,
    governor_avatar: Option<String>,
    default: bool,
}

pub async fn post(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    let payload: Value =
        serde_json::from_slice(&body).map_err(|_| ApiError::bad_request("Invalid JSON body"))?;
    let governor_id = parse_positive_governor_id_from_json(payload.get("governorId"))
        .ok_or_else(|| ApiError::bad_request("Invalid governorId"))?;

    let claims = state.reports_store.claimed_governors_collection();

    let existing_claim = claims
        .find_one(doc! { "governorId": governor_id })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    if existing_claim.is_some() {
        return Err(ApiError::conflict("Governor already claimed"));
    }

    let current_claims = claims
        .count_documents(doc! { "discordId": &session.user.discord_id })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    if current_claims >= MAX_GOVERNOR_BINDS {
        return Err(ApiError::conflict("Claim limit reached"));
    }

    let default = claims
        .find_one(doc! { "discordId": &session.user.discord_id, "default": true })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .is_none();

    let snapshot =
        find_latest_sender_snapshot(state.reports_store.battle_collection(), governor_id).await?;
    let governor_name = snapshot
        .as_ref()
        .and_then(|sender_snapshot| sender_snapshot.governor_name.clone());
    let governor_avatar = snapshot
        .as_ref()
        .and_then(|sender_snapshot| sender_snapshot.governor_avatar.clone());

    claims
        .insert_one(doc! {
            "discordId": &session.user.discord_id,
            "governorId": governor_id,
            "createdAt": DateTime::now(),
            "governorName": governor_name.clone(),
            "governorAvatar": governor_avatar.clone(),
            "default": default,
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok((
        StatusCode::OK,
        [("Cache-Control", "no-store")],
        Json(BindGovernorResponse {
            claim: ClaimedGovernor {
                governor_id,
                governor_name,
                governor_avatar,
                default,
            },
        }),
    ))
}

pub async fn patch_default(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_positive_governor_id_from_query(&params)
        .ok_or_else(|| ApiError::bad_request("Invalid governorId"))?;

    let claims = state.reports_store.claimed_governors_collection();
    let target = claims
        .find_one(doc! { "discordId": &session.user.discord_id, "governorId": governor_id })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    if target.is_none() {
        return Err(ApiError::not_found("Claim not found"));
    }

    set_bind_as_default(claims, &session.user.discord_id, governor_id).await?;

    Ok((StatusCode::NO_CONTENT, [("Cache-Control", "no-store")]))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_positive_governor_id_from_query(&params)
        .ok_or_else(|| ApiError::bad_request("Invalid governorId"))?;

    let claims = state.reports_store.claimed_governors_collection();
    let deleted_claim = claims
        .find_one_and_delete(
            doc! { "discordId": &session.user.discord_id, "governorId": governor_id },
        )
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let Some(deleted_claim) = deleted_claim else {
        return Err(ApiError::not_found("Claim not found"));
    };

    if claim_document_is_default(&deleted_claim) {
        let most_recent_governor_id =
            find_most_recent_governor_id(claims, &session.user.discord_id).await?;
        if let Some(most_recent_governor_id) = most_recent_governor_id {
            set_bind_as_default(claims, &session.user.discord_id, most_recent_governor_id).await?;
        }
    }

    Ok((StatusCode::NO_CONTENT, [("Cache-Control", "no-store")]))
}

fn claim_document_is_default(claim: &Document) -> bool {
    claim.get_bool("default").unwrap_or(false)
}

async fn set_bind_as_default(
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

async fn find_most_recent_governor_id(
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

    Ok(most_recent
        .as_ref()
        .and_then(|claim| claim.get("governorId"))
        .and_then(bson_to_i64_exact))
}

#[cfg(test)]
mod tests {
    use mongodb::bson::Bson;

    use super::*;

    #[test]
    fn reads_default_flag_from_claim_document() {
        let with_default = doc! { "default": true };
        let without_default = doc! { "governorId": 10 };
        assert!(claim_document_is_default(&with_default));
        assert!(!claim_document_is_default(&without_default));
    }

    #[test]
    fn converts_supported_bson_number_types_to_i64() {
        assert_eq!(bson_to_i64_exact(&Bson::Int32(12)), Some(12));
        assert_eq!(bson_to_i64_exact(&Bson::Int64(34)), Some(34));
        assert_eq!(bson_to_i64_exact(&Bson::Double(56.0)), Some(56));
        assert_eq!(bson_to_i64_exact(&Bson::Double(56.1)), None);
        assert_eq!(bson_to_i64_exact(&Bson::Null), None);
    }
}
