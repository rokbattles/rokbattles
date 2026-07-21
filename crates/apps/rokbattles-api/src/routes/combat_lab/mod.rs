use std::{collections::HashMap, sync::Arc};

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use futures::TryStreamExt;
use mongodb::{
    bson::{Bson, DateTime, Document, doc, from_document},
    options::{FindOneOptions, FindOptions},
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

/// Returns DRASTC rankings for every scored commander pairing.
pub async fn get_rankings(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let request = parse_rankings_request(&params)?;
    let collection = state
        .reports_store
        .precomputed_commander_pairings_collection()
        .clone_with_type::<RawCombatLabRankingDocument>();
    let cursor = collection
        .find(rankings_filter())
        .with_options(rankings_find_options(request))
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let documents: Vec<RawCombatLabRankingDocument> =
        cursor.try_collect().await.map_err(|error| ApiError::internal(error.to_string()))?;
    let response = CombatLabRankingsResponse::from(documents);

    Ok((StatusCode::OK, [("Cache-Control", "public, max-age=3600")], Json(response)))
}

fn pairing_find_options() -> FindOneOptions {
    FindOneOptions::builder().projection(doc! { "_id": 0 }).build()
}

fn rankings_filter() -> Document {
    doc! { "drastc": { "$ne": Bson::Null } }
}

fn rankings_find_options(request: RankingsRequest) -> FindOptions {
    let mut sort = Document::new();
    sort.insert(request.sort_by.mongo_path(), request.direction.mongo_order());
    sort.insert("primary_commander_id", 1);
    sort.insert("secondary_commander_id", 1);

    FindOptions::builder().projection(rankings_projection()).sort(sort).build()
}

fn rankings_projection() -> Document {
    doc! {
        "_id": 0,
        "primary_commander_id": 1,
        "secondary_commander_id": 1,
        "refreshed_at": 1,
        "drastc.overall": 1,
        "drastc.confidence": 1,
        "drastc.breakdown.damage.score": 1,
        "drastc.breakdown.rage.score": 1,
        "drastc.breakdown.assist.score": 1,
        "drastc.breakdown.sustainability.score": 1,
        "drastc.breakdown.trade.score": 1,
        "drastc.breakdown.consistency.score": 1,
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RankingsRequest {
    sort_by: RankingsSort,
    direction: RankingsDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RankingsSort {
    Overall,
    Damage,
    Rage,
    Assist,
    Sustainability,
    Trade,
    Consistency,
}

impl RankingsSort {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "overall" => Some(Self::Overall),
            "damage" => Some(Self::Damage),
            "rage" => Some(Self::Rage),
            "assist" => Some(Self::Assist),
            "sustainability" => Some(Self::Sustainability),
            "trade" => Some(Self::Trade),
            "consistency" => Some(Self::Consistency),
            _ => None,
        }
    }

    fn mongo_path(self) -> &'static str {
        match self {
            Self::Overall => "drastc.overall",
            Self::Damage => "drastc.breakdown.damage.score",
            Self::Rage => "drastc.breakdown.rage.score",
            Self::Assist => "drastc.breakdown.assist.score",
            Self::Sustainability => "drastc.breakdown.sustainability.score",
            Self::Trade => "drastc.breakdown.trade.score",
            Self::Consistency => "drastc.breakdown.consistency.score",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RankingsDirection {
    Ascending,
    Descending,
}

impl RankingsDirection {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "asc" => Some(Self::Ascending),
            "desc" => Some(Self::Descending),
            _ => None,
        }
    }

    fn mongo_order(self) -> i32 {
        match self {
            Self::Ascending => 1,
            Self::Descending => -1,
        }
    }
}

fn parse_rankings_request(params: &HashMap<String, String>) -> Result<RankingsRequest, ApiError> {
    let sort_by = params.get("sort").map_or(Ok(RankingsSort::Overall), |value| {
        RankingsSort::parse(value.trim()).ok_or_else(|| ApiError::bad_request("Invalid sort"))
    })?;
    let direction = params.get("direction").map_or(Ok(RankingsDirection::Descending), |value| {
        RankingsDirection::parse(value.trim())
            .ok_or_else(|| ApiError::bad_request("Invalid direction"))
    })?;

    Ok(RankingsRequest { sort_by, direction })
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

#[derive(Debug, Clone, Deserialize)]
struct RawCombatLabRankingDocument {
    primary_commander_id: i64,
    secondary_commander_id: i64,
    refreshed_at: DateTime,
    drastc: RawCombatLabRankingScore,
}

#[derive(Debug, Clone, Deserialize)]
struct RawCombatLabRankingScore {
    breakdown: RawCombatLabRankingBreakdown,
    overall: f64,
    confidence: DrastcConfidence,
}

#[derive(Debug, Clone, Deserialize)]
struct RawCombatLabRankingBreakdown {
    damage: RawCombatLabRankingCategory,
    rage: RawCombatLabRankingCategory,
    assist: RawCombatLabRankingCategory,
    sustainability: RawCombatLabRankingCategory,
    trade: RawCombatLabRankingCategory,
    consistency: RawCombatLabRankingCategory,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct RawCombatLabRankingCategory {
    score: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CombatLabRankingsResponse {
    items: Vec<CombatLabRankingDocument>,
    refreshed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CombatLabRankingDocument {
    primary_commander_id: i64,
    secondary_commander_id: i64,
    drastc: CombatLabRankingScore,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CombatLabRankingScore {
    breakdown: CombatLabRankingBreakdown,
    overall: f64,
    confidence: DrastcConfidence,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct CombatLabRankingBreakdown {
    damage: f64,
    rage: f64,
    assist: f64,
    sustainability: f64,
    trade: f64,
    consistency: f64,
}

impl From<RawCombatLabRankingDocument> for CombatLabRankingDocument {
    fn from(value: RawCombatLabRankingDocument) -> Self {
        let breakdown = value.drastc.breakdown;

        Self {
            primary_commander_id: value.primary_commander_id,
            secondary_commander_id: value.secondary_commander_id,
            drastc: CombatLabRankingScore {
                breakdown: CombatLabRankingBreakdown {
                    damage: breakdown.damage.score,
                    rage: breakdown.rage.score,
                    assist: breakdown.assist.score,
                    sustainability: breakdown.sustainability.score,
                    trade: breakdown.trade.score,
                    consistency: breakdown.consistency.score,
                },
                overall: value.drastc.overall,
                confidence: value.drastc.confidence,
            },
        }
    }
}

impl From<Vec<RawCombatLabRankingDocument>> for CombatLabRankingsResponse {
    fn from(documents: Vec<RawCombatLabRankingDocument>) -> Self {
        let refreshed_at =
            documents.iter().map(|document| document.refreshed_at).max().map(date_time_to_string);
        let items = documents.into_iter().map(Into::into).collect();

        Self { items, refreshed_at }
    }
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
    fn rankings_request_defaults_to_overall_descending() {
        let request = parse_rankings_request(&HashMap::new()).expect("default rankings request");

        assert_eq!(
            request,
            RankingsRequest {
                sort_by: RankingsSort::Overall,
                direction: RankingsDirection::Descending,
            }
        );
        assert_eq!(
            rankings_find_options(request).sort,
            Some(doc! {
                "drastc.overall": -1,
                "primary_commander_id": 1,
                "secondary_commander_id": 1,
            })
        );
    }

    #[test]
    fn rankings_request_supports_every_score_in_both_directions() {
        let sort_cases = [
            ("overall", RankingsSort::Overall, "drastc.overall"),
            ("damage", RankingsSort::Damage, "drastc.breakdown.damage.score"),
            ("rage", RankingsSort::Rage, "drastc.breakdown.rage.score"),
            ("assist", RankingsSort::Assist, "drastc.breakdown.assist.score"),
            (
                "sustainability",
                RankingsSort::Sustainability,
                "drastc.breakdown.sustainability.score",
            ),
            ("trade", RankingsSort::Trade, "drastc.breakdown.trade.score"),
            ("consistency", RankingsSort::Consistency, "drastc.breakdown.consistency.score"),
        ];
        let direction_cases =
            [("asc", RankingsDirection::Ascending, 1), ("desc", RankingsDirection::Descending, -1)];

        for (sort_value, expected_sort, expected_path) in sort_cases {
            for (direction_value, expected_direction, expected_order) in direction_cases {
                let params = HashMap::from([
                    ("sort".to_string(), sort_value.to_string()),
                    ("direction".to_string(), direction_value.to_string()),
                ]);
                let request = parse_rankings_request(&params).expect("supported rankings request");
                let options = rankings_find_options(request);

                assert_eq!(request.sort_by, expected_sort);
                assert_eq!(request.direction, expected_direction);
                assert_eq!(
                    options.sort.and_then(|sort| sort.get_i32(expected_path).ok()),
                    Some(expected_order)
                );
            }
        }
    }

    #[test]
    fn rankings_request_rejects_confidence_and_invalid_directions() {
        let confidence = HashMap::from([("sort".to_string(), "confidence".to_string())]);
        let invalid_direction = HashMap::from([("direction".to_string(), "sideways".to_string())]);

        assert_eq!(
            parse_rankings_request(&confidence)
                .expect_err("confidence is not sortable")
                .to_string(),
            "Invalid sort"
        );
        assert_eq!(
            parse_rankings_request(&invalid_direction).expect_err("invalid direction").to_string(),
            "Invalid direction"
        );
    }

    #[test]
    fn rankings_query_filters_scored_pairings_and_projects_required_fields() {
        assert_eq!(rankings_filter(), doc! { "drastc": { "$ne": Bson::Null } });
        assert_eq!(
            rankings_projection(),
            doc! {
                "_id": 0,
                "primary_commander_id": 1,
                "secondary_commander_id": 1,
                "refreshed_at": 1,
                "drastc.overall": 1,
                "drastc.confidence": 1,
                "drastc.breakdown.damage.score": 1,
                "drastc.breakdown.rage.score": 1,
                "drastc.breakdown.assist.score": 1,
                "drastc.breakdown.sustainability.score": 1,
                "drastc.breakdown.trade.score": 1,
                "drastc.breakdown.consistency.score": 1,
            }
        );
    }

    #[test]
    fn ranking_response_contains_ids_scores_and_confidence_only() {
        let category = doc! { "score": 5.0 };
        let raw = from_document::<RawCombatLabRankingDocument>(doc! {
            "primary_commander_id": 595_i64,
            "secondary_commander_id": 596_i64,
            "refreshed_at": DateTime::from_millis(1_000),
            "drastc": {
                "overall": 6.89,
                "confidence": {
                    "score": 4.09,
                    "unique_governors": 816_i64,
                    "effective_governors": 28.414381,
                },
                "breakdown": {
                    "damage": category.clone(),
                    "rage": category.clone(),
                    "assist": category.clone(),
                    "sustainability": category.clone(),
                    "trade": category.clone(),
                    "consistency": category,
                },
            },
        })
        .expect("projected ranking document");
        let response = to_document(&CombatLabRankingDocument::from(raw)).expect("ranking response");
        let drastc = response.get_document("drastc").expect("drastc response");
        let breakdown = drastc.get_document("breakdown").expect("breakdown response");

        assert_eq!(
            response.keys().map(String::as_str).collect::<Vec<_>>(),
            ["primaryCommanderId", "secondaryCommanderId", "drastc"]
        );
        assert_eq!(
            drastc.keys().map(String::as_str).collect::<Vec<_>>(),
            ["breakdown", "overall", "confidence"]
        );
        assert_eq!(
            breakdown.keys().map(String::as_str).collect::<Vec<_>>(),
            ["damage", "rage", "assist", "sustainability", "trade", "consistency"]
        );
        assert_eq!(breakdown.get_f64("damage"), Ok(5.0));
        assert!(response.get("strategies").is_none());
        assert!(response.get("refreshedAt").is_none());
    }

    #[test]
    fn rankings_response_uses_latest_refresh_time() {
        let ranking = |refreshed_at| RawCombatLabRankingDocument {
            primary_commander_id: 595,
            secondary_commander_id: 596,
            refreshed_at,
            drastc: RawCombatLabRankingScore {
                breakdown: RawCombatLabRankingBreakdown {
                    damage: RawCombatLabRankingCategory { score: 5.0 },
                    rage: RawCombatLabRankingCategory { score: 5.0 },
                    assist: RawCombatLabRankingCategory { score: 5.0 },
                    sustainability: RawCombatLabRankingCategory { score: 5.0 },
                    trade: RawCombatLabRankingCategory { score: 5.0 },
                    consistency: RawCombatLabRankingCategory { score: 5.0 },
                },
                overall: 5.0,
                confidence: DrastcConfidence {
                    score: 5.0,
                    unique_governors: 1,
                    effective_governors: 1.0,
                },
            },
        };
        let latest = DateTime::from_millis(2_000);

        let response = CombatLabRankingsResponse::from(vec![
            ranking(DateTime::from_millis(1_000)),
            ranking(latest),
        ]);

        assert_eq!(response.refreshed_at, Some(date_time_to_string(latest)));
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
