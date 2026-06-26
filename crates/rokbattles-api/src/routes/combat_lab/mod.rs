use std::{collections::HashMap, sync::Arc};

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use mongodb::{
    bson::{DateTime, doc, from_document},
    options::FindOneOptions,
};
use serde::{Deserialize, Serialize};

use crate::{error::ApiError, state::AppState};

/// Returns one precomputed legendary commander pairing summary for Combat Lab.
pub async fn get_pairing(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let request = parse_pairing_request(&params)?;
    let filter = doc! {
        "primary_commander_id": request.primary_commander_id,
        "secondary_commander_id": request.secondary_commander_id,
    };
    let options = FindOneOptions::builder().projection(doc! { "_id": 0 }).build();
    let Some(document) = state
        .reports_store
        .precomputed_commander_pairings_collection()
        .find_one(filter)
        .with_options(options)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
    else {
        return Err(ApiError::not_found("pairing not found"));
    };

    let response: CombatLabPairingDocument = from_document::<RawCombatLabPairingDocument>(document)
        .map_err(|error| ApiError::internal(format!("invalid combat lab document: {error}")))?
        .into();

    Ok((StatusCode::OK, [("Cache-Control", "public, max-age=3600")], Json(response)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PairingRequest {
    primary_commander_id: i64,
    secondary_commander_id: i64,
}

fn parse_pairing_request(params: &HashMap<String, String>) -> Result<PairingRequest, ApiError> {
    let primary_commander_id = parse_required_i64(params, "primaryCommanderId")?;
    let secondary_commander_id = parse_required_i64(params, "secondaryCommanderId")?;

    if primary_commander_id == secondary_commander_id {
        return Err(ApiError::bad_request("Commanders must be different"));
    }

    Ok(PairingRequest { primary_commander_id, secondary_commander_id })
}

fn parse_required_i64(params: &HashMap<String, String>, key: &str) -> Result<i64, ApiError> {
    let Some(raw) = params.get(key).map(|value| value.trim()).filter(|value| !value.is_empty())
    else {
        return Err(ApiError::bad_request(format!("Missing {key}")));
    };

    let value = raw.parse::<i64>().map_err(|_| ApiError::bad_request(format!("Invalid {key}")))?;
    if value <= 0 {
        return Err(ApiError::bad_request(format!("Invalid {key}")));
    }

    Ok(value)
}

fn date_time_to_string(value: DateTime) -> String {
    value.try_to_rfc3339_string().unwrap_or_else(|_| value.timestamp_millis().to_string())
}

#[derive(Debug, Clone, Deserialize)]
struct RawCombatLabPairingDocument {
    primary_commander_id: i64,
    secondary_commander_id: i64,
    summary: CombatLabSummary,
    drastc: Option<DrastcScore>,
    refreshed_at: DateTime,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CombatLabPairingDocument {
    primary_commander_id: i64,
    secondary_commander_id: i64,
    summary: CombatLabSummary,
    drastc: Option<DrastcScore>,
    refreshed_at: String,
}

impl From<RawCombatLabPairingDocument> for CombatLabPairingDocument {
    fn from(value: RawCombatLabPairingDocument) -> Self {
        Self {
            primary_commander_id: value.primary_commander_id,
            secondary_commander_id: value.secondary_commander_id,
            summary: value.summary,
            drastc: value.drastc,
            refreshed_at: date_time_to_string(value.refreshed_at),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all(deserialize = "snake_case", serialize = "camelCase"))]
struct CombatLabSummary {
    total_battles: i64,
    kill_points_gained: i64,
    kill_points_lost: i64,
    avg_trade_percentage: f64,
    weighted_trade_percentage: f64,
    avg_battle_duration: f64,
    total_battle_duration: i64,
    severely_wounded_inflicted: i64,
    severely_wounded_taken: i64,
    dps: f64,
    sps: f64,
    tps: f64,
    hps: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DrastcScore {
    samples: i64,
    breakdown: DrastcCategories,
    overall: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DrastcCategories {
    damage: CategoryScore,
    rage: CategoryScore,
    assist: CategoryScore,
    sustainability: CategoryScore,
    trade: CategoryScore,
    consistency: CategoryScore,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CategoryScore {
    value: f64,
    p10: f64,
    p90: f64,
    score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pairing_request_requires_both_commanders() {
        let err = parse_pairing_request(&HashMap::new()).expect_err("missing primary");
        assert_eq!(err.to_string(), "Missing primaryCommanderId");
    }

    #[test]
    fn parse_pairing_request_rejects_duplicate_commanders() {
        let params = HashMap::from([
            ("primaryCommanderId".to_string(), "579".to_string()),
            ("secondaryCommanderId".to_string(), "579".to_string()),
        ]);

        let err = parse_pairing_request(&params).expect_err("duplicate commanders");
        assert_eq!(err.to_string(), "Commanders must be different");
    }

    #[test]
    fn parse_pairing_request_accepts_valid_ids() {
        let params = HashMap::from([
            ("primaryCommanderId".to_string(), "579".to_string()),
            ("secondaryCommanderId".to_string(), "575".to_string()),
        ]);

        let request = parse_pairing_request(&params).expect("request");
        assert_eq!(
            request,
            PairingRequest { primary_commander_id: 579, secondary_commander_id: 575 }
        );
    }
}
