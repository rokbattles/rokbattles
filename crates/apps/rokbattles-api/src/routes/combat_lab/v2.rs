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
    bson::{Bson, Document, doc},
    options::FindOneOptions,
};
use rokbattles_bson::{bson_to_f64, bson_to_i64};
use serde::{Deserialize, Serialize};

use super::{CategoryScore, DrastcCategories, DrastcConfidence, DrastcScore, parse_required_i64};
use crate::{error::ApiError, state::AppState};

const ARMAMENTS_YAML: &str = include_str!("../../../../../../datasets/armaments.yaml");
static ARMAMENT_MAXIMUMS: LazyLock<Option<BTreeMap<i64, f64>>> =
    LazyLock::new(|| read_armament_maximums().ok());

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
        match kind {
            1 => performance.extend(values.iter().cloned()),
            2 => loadouts.extend(values.iter().cloned()),
            _ => {}
        }
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
}
