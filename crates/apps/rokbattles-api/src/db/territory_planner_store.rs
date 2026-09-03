//! MongoDB access for Territory Planner map configuration.

use std::collections::BTreeMap;

use futures::TryStreamExt;
use mongodb::{Collection, IndexModel, bson::doc, options::IndexOptions};
use serde::{Deserialize, Serialize};

/// A map entry returned by the Territory Planner list endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerritoryPlannerMapSummary {
    pub slug: String,
    pub title: String,
    pub order: i32,
    pub ruleset: TerritoryPlannerRuleset,
    pub supports_horse: bool,
}

/// The placement ruleset used by a planner map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerritoryPlannerRuleset {
    Home,
    LostLand,
}

/// One map's complete Territory Planner configuration.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerritoryPlannerConfig {
    pub schema_version: i32,
    pub slug: String,
    pub title: String,
    pub order: i32,
    pub ruleset: TerritoryPlannerRuleset,
    pub supports_horse: bool,
    pub image_file: String,
    pub native_map_size: i32,
    pub image_bounds: TerritoryPlannerImageBounds,
    pub spatial: TerritoryPlannerSpatialConfig,
    pub buildings: BTreeMap<String, TerritoryPlannerBuildingConfig>,
    pub resource_production_per_hour: BTreeMap<String, i64>,
    pub costs: BTreeMap<String, Vec<TerritoryPlannerCostTier>>,
}

/// Native coordinate bounds represented by the supplied map raster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerritoryPlannerImageBounds {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

/// Spatial delivery settings for a planner map.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerritoryPlannerSpatialConfig {
    pub chunk_size: i32,
    pub chunk_buffer: i32,
    pub province: bool,
    pub chunks: Vec<[i32; 2]>,
}

/// Per-alliance limit for one building kind.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerritoryPlannerBuildingConfig {
    pub limit: i32,
}

/// A contiguous sequence of buildings that share the same construction cost.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerritoryPlannerCostTier {
    pub from: i32,
    pub to: i32,
    pub cost: BTreeMap<String, i64>,
}

/// Version-independent Territory Planner map configuration store.
#[derive(Debug, Clone)]
pub struct TerritoryPlannerStore {
    maps: Collection<TerritoryPlannerConfig>,
}

impl TerritoryPlannerStore {
    /// Create a store backed by the Territory Planner collection.
    pub fn new(db: mongodb::Database) -> Self {
        Self { maps: db.collection("g_rok_territory_planner") }
    }

    /// Ensure each planner slug identifies exactly one map document.
    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        self.maps
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "slug": 1 })
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
            )
            .await?;
        Ok(())
    }

    /// Return all planner maps in display order.
    pub async fn list_maps(&self) -> mongodb::error::Result<Vec<TerritoryPlannerMapSummary>> {
        let cursor = self
            .maps
            .clone_with_type::<TerritoryPlannerMapSummary>()
            .find(doc! {})
            .projection(doc! {
                "_id": 0,
                "slug": 1,
                "title": 1,
                "order": 1,
                "ruleset": 1,
                "supportsHorse": 1,
            })
            .sort(doc! { "order": 1 })
            .await?;
        cursor.try_collect().await
    }

    /// Return one planner map by slug.
    pub async fn find_map(
        &self,
        slug: &str,
    ) -> mongodb::error::Result<Option<TerritoryPlannerConfig>> {
        self.maps.find_one(doc! { "slug": slug }).projection(doc! { "_id": 0 }).await
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{doc, from_document, oid::ObjectId, to_document};

    use super::*;

    #[test]
    fn map_summary_should_ignore_mongo_id_when_deserializing() {
        let document = doc! {
            "_id": ObjectId::new(),
            "slug": "s20-song-of-troy",
            "title": "Season 20: Song of Troy",
            "order": 16,
            "ruleset": "lost-land",
            "supportsHorse": true,
        };

        let summary = from_document::<TerritoryPlannerMapSummary>(document);

        assert!(summary.is_ok(), "map summary should accept Mongo's generated _id: {summary:?}");
    }

    #[test]
    fn map_summary_should_serialize_camel_case_fields() {
        let summary = TerritoryPlannerMapSummary {
            slug: "s20-song-of-troy".to_owned(),
            title: "Season 20: Song of Troy".to_owned(),
            order: 16,
            ruleset: TerritoryPlannerRuleset::LostLand,
            supports_horse: true,
        };

        let document = to_document(&summary).expect("summary should serialize");

        assert_eq!(document.get_bool("supportsHorse"), Ok(true));
    }
}
