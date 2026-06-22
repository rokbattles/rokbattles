use std::{collections::HashMap, sync::Arc};

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use futures::StreamExt;
use mongodb::{
    Collection,
    bson::{DateTime, Document, doc, from_document},
    options::FindOptions,
};
use serde::{Deserialize, Serialize};

use crate::{error::ApiError, state::AppState};

const LOOT_EXPLORER_CACHE_CONTROL: &str = "public, max-age=300";

/// Returns precomputed barbarian loot documents.
pub async fn get_barbarians(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let request = parse_level_request(&params)?;
    let items = fetch_documents::<RawBarbarianDocument, BarbarianDocument>(
        state.reports_store.precomputed_barbarian_collection(),
        request.filter(),
        doc! { "kind": 1, "level": 1 },
    )
    .await?;

    Ok((
        StatusCode::OK,
        [("Cache-Control", LOOT_EXPLORER_CACHE_CONTROL)],
        Json(BarbarianResponse { items }),
    ))
}

/// Returns precomputed barbarian fort loot documents.
pub async fn get_barbarian_forts(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let request = parse_level_request(&params)?;
    let items = fetch_documents::<RawBarbarianFortDocument, BarbarianFortDocument>(
        state.reports_store.precomputed_barbarian_fort_collection(),
        request.filter(),
        doc! { "kind": 1, "level": 1 },
    )
    .await?;

    Ok((
        StatusCode::OK,
        [("Cache-Control", LOOT_EXPLORER_CACHE_CONTROL)],
        Json(BarbarianFortResponse { items }),
    ))
}

/// Returns precomputed Baulur loot documents.
pub async fn get_baulurs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let request = parse_kind_request(&params)?;
    let items = fetch_documents::<RawBaulurDocument, BaulurDocument>(
        state.reports_store.precomputed_baulur_collection(),
        request.filter(),
        doc! { "kind": 1 },
    )
    .await?;

    Ok((
        StatusCode::OK,
        [("Cache-Control", LOOT_EXPLORER_CACHE_CONTROL)],
        Json(BaulurResponse { items }),
    ))
}

async fn fetch_documents<Raw, Output>(
    collection: &Collection<Document>,
    filter: Document,
    sort: Document,
) -> Result<Vec<Output>, ApiError>
where
    Raw: for<'de> Deserialize<'de> + Into<Output>,
{
    let options = FindOptions::builder().projection(doc! { "_id": 0 }).sort(sort).build();
    let mut cursor = collection
        .find(filter)
        .with_options(options)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let mut items = Vec::new();

    while let Some(next) = cursor.next().await {
        let document = next.map_err(|error| ApiError::internal(error.to_string()))?;
        let item = from_document::<Raw>(document)
            .map_err(|error| {
                ApiError::internal(format!("invalid loot explorer document: {error}"))
            })?
            .into();
        items.push(item);
    }

    Ok(items)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LevelRequest {
    kind: Option<i32>,
    levels: Option<Vec<i32>>,
}

impl LevelRequest {
    fn filter(&self) -> Document {
        let mut filter = Document::new();
        if let Some(kind) = self.kind {
            filter.insert("kind", kind);
        }
        if let Some(levels) = &self.levels {
            filter.insert("level", doc! { "$in": levels });
        }

        filter
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KindRequest {
    kind: Option<i32>,
}

impl KindRequest {
    fn filter(&self) -> Document {
        self.kind.map(|kind| doc! { "kind": kind }).unwrap_or_default()
    }
}

fn parse_level_request(params: &HashMap<String, String>) -> Result<LevelRequest, ApiError> {
    Ok(LevelRequest {
        kind: parse_optional_i32(params, "kind")?,
        levels: parse_optional_i32_list(params, "level")?,
    })
}

fn parse_kind_request(params: &HashMap<String, String>) -> Result<KindRequest, ApiError> {
    Ok(KindRequest { kind: parse_optional_i32(params, "kind")? })
}

fn parse_optional_i32(
    params: &HashMap<String, String>,
    key: &str,
) -> Result<Option<i32>, ApiError> {
    let Some(raw) = params.get(key).map(|value| value.trim()).filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };

    raw.parse::<i32>().map(Some).map_err(|_| ApiError::bad_request(format!("Invalid {key}")))
}

fn parse_optional_i32_list(
    params: &HashMap<String, String>,
    key: &str,
) -> Result<Option<Vec<i32>>, ApiError> {
    let Some(raw) = params.get(key).map(|value| value.trim()).filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };

    let mut parsed = Vec::new();
    for value in raw.split(',').map(str::trim).filter(|value| !value.is_empty()) {
        let parsed_value =
            value.parse::<i32>().map_err(|_| ApiError::bad_request(format!("Invalid {key}")))?;
        if !parsed.contains(&parsed_value) {
            parsed.push(parsed_value);
        }
    }

    if parsed.is_empty() {
        return Err(ApiError::bad_request(format!("Invalid {key}")));
    }

    Ok(Some(parsed))
}

fn date_time_to_string(value: DateTime) -> String {
    value.try_to_rfc3339_string().unwrap_or_else(|_| value.timestamp_millis().to_string())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BarbarianResponse {
    items: Vec<BarbarianDocument>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BarbarianFortResponse {
    items: Vec<BarbarianFortDocument>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BaulurResponse {
    items: Vec<BaulurDocument>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawBarbarianDocument {
    kind: i32,
    level: i32,
    loot: Vec<LootDrop>,
    data: BarbarianData,
    totals: BarbarianTotals,
    refreshed_at: DateTime,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BarbarianDocument {
    kind: i32,
    level: i32,
    loot: Vec<LootDrop>,
    data: BarbarianData,
    totals: BarbarianTotals,
    refreshed_at: String,
}

impl From<RawBarbarianDocument> for BarbarianDocument {
    fn from(value: RawBarbarianDocument) -> Self {
        Self {
            kind: value.kind,
            level: value.level,
            loot: value.loot,
            data: value.data,
            totals: value.totals,
            refreshed_at: date_time_to_string(value.refreshed_at),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawBarbarianFortDocument {
    kind: i32,
    level: i32,
    reward_tiers: Vec<RewardTier>,
    data: FortData,
    totals: FortTotals,
    refreshed_at: DateTime,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BarbarianFortDocument {
    kind: i32,
    level: i32,
    reward_tiers: Vec<RewardTier>,
    data: FortData,
    totals: FortTotals,
    refreshed_at: String,
}

impl From<RawBarbarianFortDocument> for BarbarianFortDocument {
    fn from(value: RawBarbarianFortDocument) -> Self {
        Self {
            kind: value.kind,
            level: value.level,
            reward_tiers: value.reward_tiers,
            data: value.data,
            totals: value.totals,
            refreshed_at: date_time_to_string(value.refreshed_at),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawBaulurDocument {
    kind: i32,
    loot_pools: Vec<DamageLootPool>,
    totals: BaulurTotals,
    refreshed_at: DateTime,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BaulurDocument {
    kind: i32,
    loot_pools: Vec<DamageLootPool>,
    totals: BaulurTotals,
    refreshed_at: String,
}

impl From<RawBaulurDocument> for BaulurDocument {
    fn from(value: RawBaulurDocument) -> Self {
        Self {
            kind: value.kind,
            loot_pools: value.loot_pools,
            totals: value.totals,
            refreshed_at: date_time_to_string(value.refreshed_at),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct BarbarianData {
    b_type: i32,
    ap_cost: i32,
    honor_points: i32,
    base_xp: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct FortData {
    ap_cost: i32,
    honor_points: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct BarbarianTotals {
    results: i64,
    ap_used: i64,
    honor_points_gained: i64,
    xp_gained: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct FortTotals {
    results: i64,
    ap_used: i64,
    honor_points_gained: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct BaulurTotals {
    results: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct RewardTier {
    tier: i32,
    results: i64,
    receive_rate: f64,
    damage_percentage: NumericRange,
    loot: Vec<LootDrop>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct DamageLootPool {
    pool: i32,
    results: i64,
    receive_rate: f64,
    damage_factor: NumericRange,
    loot: Vec<LootDrop>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct LootDrop {
    #[serde(rename = "type")]
    reward_type: i32,
    sub_type: i32,
    results: i64,
    drop_rate: f64,
    quantity: QuantityRange,
    total_quantity: i64,
    average_quantity: f64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct QuantityRange {
    min: i64,
    max: i64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct NumericRange {
    min: Option<f64>,
    max: Option<f64>,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{parse_kind_request, parse_level_request};

    #[test]
    fn parse_level_request_accepts_kind_and_comma_separated_levels() {
        let params = HashMap::from([
            ("kind".to_string(), "38".to_string()),
            ("level".to_string(), "38, 39,38".to_string()),
        ]);

        let request = parse_level_request(&params).expect("request should parse");

        assert_eq!(request.kind, Some(38));
        assert_eq!(request.levels, Some(vec![38, 39]));
    }

    #[test]
    fn parse_kind_request_rejects_invalid_kind() {
        let params = HashMap::from([("kind".to_string(), "abc".to_string())]);

        assert!(parse_kind_request(&params).is_err());
    }
}
