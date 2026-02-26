use std::collections::HashMap;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::{Json, http::StatusCode};
use mongodb::Collection;
use mongodb::bson::{Bson, DateTime, Document, doc};
use mongodb::options::FindOneOptions;
use serde::Serialize;
use serde_json::Value;

use crate::auth::AuthenticatedSession;
use crate::error::ApiError;
use crate::governor_bindings::snapshot::find_latest_sender_snapshot;
use crate::state::AppState;

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
    let governor_id = parse_governor_id_value(payload.get("governorId"))
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
    let governor_id = parse_governor_id_query(&params)
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
    let governor_id = parse_governor_id_query(&params)
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

fn parse_governor_id_query(params: &HashMap<String, String>) -> Option<i64> {
    let value = params.get("governorId")?;
    parse_governor_id_str(value)
}

fn parse_governor_id_value(value: Option<&Value>) -> Option<i64> {
    let value = value?;

    let parsed = match value {
        Value::Number(number) => {
            if let Some(parsed) = number.as_i64() {
                Some(parsed)
            } else if let Some(parsed) = number.as_u64() {
                i64::try_from(parsed).ok()
            } else if let Some(parsed) = number.as_f64() {
                if parsed.is_finite()
                    && parsed.fract() == 0.0
                    && parsed >= i64::MIN as f64
                    && parsed <= i64::MAX as f64
                {
                    Some(parsed as i64)
                } else {
                    None
                }
            } else {
                None
            }
        }
        Value::String(value) => parse_governor_id_str(value),
        _ => None,
    };

    parsed.filter(|governor_id| *governor_id > 0)
}

fn parse_governor_id_str(value: &str) -> Option<i64> {
    let parsed = value.trim().parse::<i64>().ok()?;
    if parsed > 0 { Some(parsed) } else { None }
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
        .and_then(bson_to_i64))
}

fn bson_to_i64(value: &Bson) -> Option<i64> {
    match value {
        Bson::Int32(value) => Some(i64::from(*value)),
        Bson::Int64(value) => Some(*value),
        Bson::Double(value) => {
            if value.is_finite()
                && value.fract() == 0.0
                && *value >= i64::MIN as f64
                && *value <= i64::MAX as f64
            {
                Some(*value as i64)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_governor_id_from_number_and_string() {
        assert_eq!(
            parse_governor_id_value(Some(&Value::Number(123.into()))),
            Some(123)
        );
        assert_eq!(
            parse_governor_id_value(Some(&Value::String("456".to_string()))),
            Some(456)
        );
    }

    #[test]
    fn rejects_invalid_or_non_positive_governor_id_values() {
        assert_eq!(parse_governor_id_value(Some(&Value::Null)), None);
        assert_eq!(
            parse_governor_id_value(Some(&Value::String("not-a-number".to_string()))),
            None
        );
        assert_eq!(
            parse_governor_id_value(Some(&Value::Number((-1).into()))),
            None
        );
        assert_eq!(
            parse_governor_id_value(Some(&Value::String("0".to_string()))),
            None
        );
    }

    #[test]
    fn parses_governor_id_from_query_params() {
        let params = HashMap::from([("governorId".to_string(), "789".to_string())]);
        assert_eq!(parse_governor_id_query(&params), Some(789));
    }

    #[test]
    fn reads_default_flag_from_claim_document() {
        let with_default = doc! { "default": true };
        let without_default = doc! { "governorId": 10 };
        assert!(claim_document_is_default(&with_default));
        assert!(!claim_document_is_default(&without_default));
    }

    #[test]
    fn converts_supported_bson_number_types_to_i64() {
        assert_eq!(bson_to_i64(&Bson::Int32(12)), Some(12));
        assert_eq!(bson_to_i64(&Bson::Int64(34)), Some(34));
        assert_eq!(bson_to_i64(&Bson::Double(56.0)), Some(56));
        assert_eq!(bson_to_i64(&Bson::Double(56.1)), None);
        assert_eq!(bson_to_i64(&Bson::Null), None);
    }
}
