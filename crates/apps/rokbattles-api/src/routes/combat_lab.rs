use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, LazyLock},
};

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use futures::TryStreamExt;
use mongodb::{
    bson::{Bson, DateTime, Document, doc},
    options::{FindOneOptions, FindOptions},
};
use rokbattles_bson::{bson_to_f64, bson_to_i64};
use serde::{Deserialize, Serialize};

use crate::{error::ApiError, state::AppState};

const ARMAMENTS_YAML: &str = include_str!("../../../../../datasets/armaments.yaml");
static ARMAMENT_MAXIMUMS: LazyLock<Option<BTreeMap<i64, f64>>> =
    LazyLock::new(|| read_armament_maximums().ok());

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

/// Return one completed compact Combat Lab generation.
pub async fn get_pairing(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let primary = parse_required_i64(&params, "primary")?;
    let secondary = parse_required_i64(&params, "secondary")?;
    if primary == secondary {
        return Err(ApiError::bad_request("Commanders must be different"));
    }

    let collection = state.reports_store.precomputed_commander_pairings_v2_collection();
    let root_options =
        FindOneOptions::builder().projection(doc! { "_id": 0 }).sort(doc! { "g": -1 }).build();
    let Some(root) = collection
        .find_one(doc! { "k": 0_i64, "p": primary, "s": secondary })
        .with_options(root_options)
        .await
        .map_err(internal_mongo)?
    else {
        return Err(ApiError::not_found("pairing not found"));
    };
    let generation = root
        .get_datetime("g")
        .copied()
        .map_err(|error| ApiError::internal(format!("invalid Combat Lab generation: {error}")))?;
    let summaries = root
        .get_array("r")
        .cloned()
        .map_err(|error| ApiError::internal(format!("invalid Combat Lab summaries: {error}")))?;
    let drastc = root.get("d").map(map_drastc).transpose()?;
    let armament_maximums = ARMAMENT_MAXIMUMS
        .as_ref()
        .ok_or_else(|| ApiError::internal("invalid embedded armament dataset"))?;

    let cursor = collection
        .find(doc! {
            "k": { "$in": [1_i64, 2_i64] },
            "p": primary,
            "s": secondary,
            "g": generation,
        })
        .projection(doc! { "_id": 0, "k": 1, "m": 1, "q": 1, "v": 1 })
        .sort(doc! { "k": 1, "m": 1, "q": 1 })
        .await
        .map_err(internal_mongo)?;
    let chunks: Vec<Document> = cursor.try_collect().await.map_err(internal_mongo)?;
    let mut performance = Vec::new();
    let mut loadouts = Vec::new();
    for chunk in chunks {
        let kind = direct_i64(&chunk, "k")
            .ok_or_else(|| ApiError::internal("compact Combat Lab chunk is missing its kind"))?;
        let values = chunk.get_array("v").map_err(|error| {
            ApiError::internal(format!("invalid compact Combat Lab chunk: {error}"))
        })?;
        append_chunk(kind, values, &mut performance, &mut loadouts);
    }

    let response = CombatLabV2Response {
        generated_at_ms: generation.timestamp_millis(),
        pairing: Pairing { primary_commander_id: primary, secondary_commander_id: secondary },
        drastc,
        summaries,
        performance,
        loadouts,
        armament_maximums,
    };
    Ok((StatusCode::OK, [("Cache-Control", "public, max-age=3600")], Json(response)))
}

/// Return DRASTC rankings from the v2 materialized score collection.
pub async fn get_rankings(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let request = parse_rankings_request(&params)?;
    let collection = state
        .reports_store
        .precomputed_drastc_collection()
        .clone_with_type::<RawCombatLabRankingDocument>();
    let cursor = collection
        .find(doc! {})
        .with_options(rankings_find_options(request))
        .await
        .map_err(internal_mongo)?;
    let documents: Vec<RawCombatLabRankingDocument> =
        cursor.try_collect().await.map_err(internal_mongo)?;

    Ok((
        StatusCode::OK,
        [("Cache-Control", "public, max-age=3600")],
        Json(CombatLabRankingsResponse::from(documents)),
    ))
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

fn append_chunk(kind: i64, values: &[Bson], performance: &mut Vec<Bson>, loadouts: &mut Vec<Bson>) {
    match kind {
        1 => performance.extend(values.iter().cloned()),
        2 => loadouts.extend(values.iter().cloned()),
        _ => {}
    }
}

fn map_drastc(value: &Bson) -> Result<DrastcScore, ApiError> {
    let values = tuple(value, "DRASTC")?;
    let categories = values
        .get(5)
        .and_then(Bson::as_array)
        .ok_or_else(|| ApiError::internal("compact DRASTC categories are missing"))?;
    let category = |index| -> Result<CategoryScore, ApiError> {
        let values = categories
            .get(index)
            .ok_or_else(|| ApiError::internal("compact DRASTC category is missing"))?;
        let values = tuple(values, "DRASTC category")?;
        Ok(CategoryScore {
            value: tuple_f64(values, 0)?,
            p10: tuple_f64(values, 1)?,
            p90: tuple_f64(values, 2)?,
            score: tuple_f64(values, 3)?,
        })
    };

    Ok(DrastcScore {
        samples: tuple_i64(values, 0)?,
        overall: tuple_f64(values, 1)?,
        confidence: DrastcConfidence {
            score: tuple_f64(values, 2)?,
            unique_governors: tuple_i64(values, 3)?,
            effective_governors: tuple_f64(values, 4)?,
        },
        breakdown: DrastcCategories {
            damage: category(0)?,
            rage: category(1)?,
            assist: category(2)?,
            sustainability: category(3)?,
            trade: category(4)?,
            consistency: category(5)?,
        },
    })
}

fn tuple<'a>(value: &'a Bson, label: &str) -> Result<&'a [Bson], ApiError> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| ApiError::internal(format!("compact {label} is not an array")))
}

fn tuple_i64(values: &[Bson], index: usize) -> Result<i64, ApiError> {
    values
        .get(index)
        .and_then(bson_to_i64)
        .ok_or_else(|| ApiError::internal(format!("compact tuple index {index} is not an integer")))
}

fn tuple_f64(values: &[Bson], index: usize) -> Result<f64, ApiError> {
    values
        .get(index)
        .and_then(bson_to_f64)
        .ok_or_else(|| ApiError::internal(format!("compact tuple index {index} is not numeric")))
}

fn direct_i64(document: &Document, key: &str) -> Option<i64> {
    document.get(key).and_then(bson_to_i64)
}

fn internal_mongo(error: mongodb::error::Error) -> ApiError {
    ApiError::internal(error.to_string())
}

fn date_time_to_string(value: DateTime) -> String {
    value.try_to_rfc3339_string().unwrap_or_else(|_| value.timestamp_millis().to_string())
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CombatLabV2Response<'a> {
    generated_at_ms: i64,
    pairing: Pairing,
    drastc: Option<DrastcScore>,
    summaries: Vec<Bson>,
    performance: Vec<Bson>,
    loadouts: Vec<Bson>,
    armament_maximums: &'a BTreeMap<i64, f64>,
}

fn read_armament_maximums() -> Result<BTreeMap<i64, f64>, yaml_serde::Error> {
    let dataset: ArmamentDataset = yaml_serde::from_str(ARMAMENTS_YAML)?;
    Ok(dataset
        .armaments
        .into_iter()
        .filter_map(|(id, definition)| {
            let maximum = definition.max_roll?;
            (id > 0 && maximum.is_finite() && maximum >= 0.0).then_some((id, maximum))
        })
        .collect())
}

#[derive(Deserialize)]
struct ArmamentDataset {
    armaments: BTreeMap<i64, ArmamentDefinition>,
}

#[derive(Deserialize)]
struct ArmamentDefinition {
    max_roll: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Pairing {
    primary_commander_id: i64,
    secondary_commander_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_drastc_maps_fixed_category_order() {
        let category = Bson::Array(vec![1.0.into(), 2.0.into(), 3.0.into(), 4.0.into()]);
        let value = Bson::Array(vec![
            10_i64.into(),
            80.0.into(),
            95.0.into(),
            20_i64.into(),
            12.5.into(),
            Bson::Array(vec![category; 6]),
        ]);

        let score = map_drastc(&value).expect("DRASTC");

        assert_eq!(score.samples, 10);
        assert_eq!(score.breakdown.consistency.score, 4.0);
        assert_eq!(score.confidence.unique_governors, 20);
    }

    #[test]
    fn armament_yaml_contains_expected_maximum_rolls() {
        let maximums = read_armament_maximums().expect("armament maximums");

        assert_eq!(maximums.get(&10_002), Some(&0.02));
    }

    #[test]
    fn loadout_chunks_preserve_additive_inner_tuple_kinds() {
        let skill_tuple = Bson::Array(vec![4_i64.into(), 1_000_i64.into(), 0_i64.into()]);
        let mut performance = Vec::new();
        let mut loadouts = Vec::new();

        append_chunk(2, std::slice::from_ref(&skill_tuple), &mut performance, &mut loadouts);

        assert!(performance.is_empty());
        assert_eq!(loadouts, vec![skill_tuple]);
    }
}
