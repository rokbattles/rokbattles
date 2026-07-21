use std::{collections::HashMap, sync::Arc};

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use mongodb::{
    bson::{DateTime, Document, doc, from_document},
    options::FindOneOptions,
};
use serde::{Deserialize, Serialize};

use crate::{error::ApiError, state::AppState};

/// Returns one precomputed legendary commander pairing for Combat Lab.
pub async fn get_pairing(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let request = parse_pairing_request(&params)?;
    let filter = doc! {
        "primary_commander_id": request.primary_commander_id,
        "secondary_commander_id": request.secondary_commander_id,
    };
    let options = pairing_find_options();
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

    let response = map_pairing_document(document)?;

    Ok((StatusCode::OK, [("Cache-Control", "public, max-age=3600")], Json(response)))
}

fn pairing_find_options() -> FindOneOptions {
    FindOneOptions::builder().projection(doc! { "_id": 0 }).build()
}

fn map_pairing_document(document: Document) -> Result<CombatLabPairingDocument, ApiError> {
    from_document::<RawCombatLabPairingDocument>(document)
        .map(Into::into)
        .map_err(|error| ApiError::internal(format!("invalid combat lab document: {error}")))
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
    strategies: CombatLabStrategies,
    drastc: Option<DrastcScore>,
    refreshed_at: DateTime,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CombatLabPairingDocument {
    primary_commander_id: i64,
    secondary_commander_id: i64,
    strategies: CombatLabStrategies,
    drastc: Option<DrastcScore>,
    refreshed_at: String,
}

impl From<RawCombatLabPairingDocument> for CombatLabPairingDocument {
    fn from(value: RawCombatLabPairingDocument) -> Self {
        Self {
            primary_commander_id: value.primary_commander_id,
            secondary_commander_id: value.secondary_commander_id,
            strategies: value.strategies,
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
#[serde(rename_all(deserialize = "snake_case", serialize = "camelCase"))]
struct CombatLabStrategies {
    all: CombatLabStrategySummary,
    open_field: CombatLabStrategySummary,
    swarming: CombatLabStrategySummary,
    rally: CombatLabStrategySummary,
    garrison: CombatLabStrategySummary,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct CombatLabStrategySummary {
    #[serde(flatten)]
    summary: CombatLabSummary,
    formations: Vec<CombatLabFormation>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
struct CombatLabFormation {
    id: i64,
    count: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DrastcScore {
    samples: i64,
    breakdown: DrastcCategories,
    overall: f64,
    confidence: DrastcConfidence,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all(deserialize = "snake_case", serialize = "camelCase"))]
struct DrastcConfidence {
    score: f64,
    unique_governors: i64,
    effective_governors: f64,
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
    use mongodb::bson::{Bson, to_document};

    use super::*;

    fn stored_summary(total_battles: i64) -> Document {
        doc! {
            "total_battles": total_battles,
            "kill_points_gained": 0_i64,
            "kill_points_lost": 0_i64,
            "avg_trade_percentage": 0.0,
            "weighted_trade_percentage": 100.0,
            "avg_battle_duration": 0.0,
            "total_battle_duration": 0_i64,
            "severely_wounded_inflicted": 0_i64,
            "severely_wounded_taken": 0_i64,
            "dps": 0.0,
            "sps": 0.0,
            "tps": 0.0,
            "hps": 0.0,
        }
    }

    fn stored_strategy(total_battles: i64, formation_id: i64) -> Document {
        let mut strategy = stored_summary(total_battles);
        strategy.insert(
            "formations",
            Bson::Array(vec![Bson::Document(doc! {
                "id": formation_id,
                "count": total_battles,
            })]),
        );
        strategy
    }

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

    #[test]
    fn pairing_find_options_excludes_mongodb_id() {
        assert_eq!(pairing_find_options().projection, Some(doc! { "_id": 0 }));
    }

    #[test]
    fn map_pairing_document_includes_all_strategies_and_formations() {
        let response = map_pairing_document(doc! {
            "_id": "excluded",
            "primary_commander_id": 509_i64,
            "secondary_commander_id": 6_i64,
            "strategies": {
                "all": stored_strategy(15, 0),
                "open_field": stored_strategy(10, 2),
                "swarming": stored_strategy(3, 1),
                "rally": stored_strategy(1, 19),
                "garrison": stored_strategy(1, 19),
            },
            "drastc": Bson::Null,
            "refreshed_at": DateTime::from_millis(0),
        })
        .expect("mapped response");
        let response = to_document(&response).expect("serialized response");
        let strategies = response.get_document("strategies").expect("strategies");
        let open_field = strategies.get_document("openField").expect("open field");

        assert_eq!(
            strategies.keys().map(String::as_str).collect::<Vec<_>>(),
            ["all", "openField", "swarming", "rally", "garrison"]
        );
        assert_eq!(open_field.get_i64("totalBattles"), Ok(10));
        assert!(open_field.get("total_battles").is_none());
        assert_eq!(
            open_field.get_array("formations"),
            Ok(&vec![Bson::Document(doc! { "id": 2_i64, "count": 10_i64 })])
        );
        assert!(response.get("_id").is_none());
    }

    #[test]
    fn map_pairing_document_rejects_documents_without_strategies() {
        let error = map_pairing_document(doc! {
            "primary_commander_id": 509_i64,
            "secondary_commander_id": 6_i64,
            "drastc": Bson::Null,
            "refreshed_at": DateTime::from_millis(0),
        })
        .expect_err("strategies are required");

        assert!(error.to_string().contains("missing field `strategies`"));
    }

    #[test]
    fn map_pairing_document_requires_and_serializes_drastc_confidence() {
        let category = doc! { "value": 1.0, "p10": 0.0, "p90": 2.0, "score": 5.0 };
        let response = map_pairing_document(doc! {
            "primary_commander_id": 595_i64,
            "secondary_commander_id": 596_i64,
            "strategies": {
                "all": stored_strategy(111_512, 0),
                "open_field": stored_strategy(111_512, 0),
                "swarming": stored_strategy(0, 0),
                "rally": stored_strategy(0, 0),
                "garrison": stored_strategy(0, 0),
            },
            "drastc": {
                "samples": 111_512_i64,
                "breakdown": {
                    "damage": category.clone(),
                    "rage": category.clone(),
                    "assist": category.clone(),
                    "sustainability": category.clone(),
                    "trade": category.clone(),
                    "consistency": category,
                },
                "overall": 6.89,
                "confidence": {
                    "score": 4.09,
                    "unique_governors": 816_i64,
                    "effective_governors": 28.414381,
                },
            },
            "refreshed_at": DateTime::from_millis(0),
        })
        .expect("mapped response");
        let response = to_document(&response).expect("serialized response");
        let confidence = response
            .get_document("drastc")
            .and_then(|drastc| drastc.get_document("confidence"))
            .expect("serialized confidence");

        assert_eq!(confidence.get_f64("score"), Ok(4.09));
        assert_eq!(confidence.get_i64("uniqueGovernors"), Ok(816));
        assert_eq!(confidence.get_f64("effectiveGovernors"), Ok(28.414381));
        assert!(confidence.get("unique_governors").is_none());
    }

    #[test]
    fn stored_drastc_score_without_confidence_is_rejected() {
        let category = doc! { "value": 1.0, "p10": 0.0, "p90": 2.0, "score": 5.0 };
        let error = from_document::<DrastcScore>(doc! {
            "samples": 1_i64,
            "breakdown": {
                "damage": category.clone(),
                "rage": category.clone(),
                "assist": category.clone(),
                "sustainability": category.clone(),
                "trade": category.clone(),
                "consistency": category,
            },
            "overall": 5.0,
        })
        .expect_err("confidence is required");

        assert!(error.to_string().contains("missing field `confidence`"));
    }
}
