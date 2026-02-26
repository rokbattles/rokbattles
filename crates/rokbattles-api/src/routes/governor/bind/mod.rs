use std::collections::HashMap;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::{Json, http::StatusCode};
use mongodb::bson::{DateTime, doc};
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
            },
        }),
    ))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_query(&params)
        .ok_or_else(|| ApiError::bad_request("Invalid governorId"))?;

    let delete_result = state
        .reports_store
        .claimed_governors_collection()
        .delete_one(doc! { "discordId": &session.user.discord_id, "governorId": governor_id })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    if delete_result.deleted_count == 0 {
        return Err(ApiError::not_found("Claim not found"));
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
}
