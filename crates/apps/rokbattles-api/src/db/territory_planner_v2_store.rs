//! MongoDB persistence for the Territory Planner v2 catalog and shared plans.
//!
//! Catalog documents combine public rendering metadata with private site records.
//! Routes remove the site records before returning catalog metadata, while share
//! validation uses them to resolve site and pass identifiers. Shared plans live in
//! a separate, insert-only collection and are addressed by server-generated IDs.

use std::collections::BTreeMap;

use futures::TryStreamExt;
use mongodb::{
    Collection, IndexModel,
    bson::{DateTime, doc},
    options::IndexOptions,
};
use serde::{Deserialize, Serialize};

use crate::routes::territory_planner_v2::TerritoryPlan;

pub const MAPS_COLLECTION: &str = "g_rok_territory_planner_v2";
pub const PLANS_COLLECTION: &str = "g_rok_territory_plans_v2";

/// A season used to group maps in the public catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerritorySeason {
    Preparation,
    #[serde(rename = "season-1")]
    Season1,
    #[serde(rename = "season-2")]
    Season2,
    #[serde(rename = "season-3")]
    Season3,
    Conquest,
}

/// The rule family that governs placement and planner modes for a map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerritoryRuleset {
    Home,
    LostLand,
}

/// Integrity metadata for one catalog-pinned CDN asset.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerritoryAsset {
    pub sha256: String,
    pub bytes: u64,
}

/// The maximum number of buildings of a kind per alliance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TerritoryBuildingRule {
    pub limit: u32,
}

/// Collision geometry associated with a structure kind.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(tag = "shape", rename_all = "kebab-case", deny_unknown_fields)]
pub enum TerritoryCollisionRule {
    /// A square centered on the unsnapped position, measured in native map units.
    WorldSquare {
        #[serde(rename = "halfSize")]
        half_size: f64,
    },
    /// A square snapped to the territory grid; radius excludes the center cell.
    TerritorySquare {
        #[serde(rename = "radiusInCells")]
        radius_in_cells: u32,
    },
}

/// Placement and ownership rules for structures found in map data.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerritoryStructureRule {
    pub collision: TerritoryCollisionRule,
    /// Ownership radius in 18-native-unit cells, producing a side of `2 * radius + 1`.
    pub territory_radius_in_cells: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teleport_radius_in_cells: Option<u32>,
    pub claimable: bool,
}

/// Resource amounts charged by one building-cost tier.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TerritoryResourceCost {
    pub food: u64,
    pub wood: u64,
    pub stone: u64,
    pub gold: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credits: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crystal: Option<u64>,
}

/// An inclusive building-number range that shares one cost.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TerritoryCostTier {
    pub from: u32,
    pub to: u32,
    pub cost: TerritoryResourceCost,
}

/// A private catalog site used to validate route references.
///
/// Sites are persisted with their catalog but are omitted from public map
/// responses. Clients obtain display geometry from integrity-checked assets.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TerritorySite {
    pub id: String,
    pub kind: String,
    pub x: f64,
    pub y: f64,
}

/// Versioned metadata and validation data for one planner map.
///
/// Most fields form the public map contract. [`Self::sites`] is server-only
/// reference data and must be removed before serializing a public response.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerritoryMapCatalog {
    pub schema_version: u8,
    pub map: String,
    pub name: String,
    pub season: TerritorySeason,
    pub order: i32,
    pub bounds: [f64; 4],
    pub pixel_size: [u32; 2],
    pub categories: Vec<String>,
    pub assets: BTreeMap<String, TerritoryAsset>,
    #[serde(default)]
    /// Sprite filenames keyed by structure configuration ID, not structure category.
    pub structure_icons: BTreeMap<String, String>,
    #[serde(default)]
    pub building_icons: BTreeMap<String, String>,
    #[serde(default)]
    pub site_icons: BTreeMap<String, String>,
    pub buildings: BTreeMap<String, TerritoryBuildingRule>,
    #[serde(default)]
    pub structure_rules: BTreeMap<String, TerritoryStructureRule>,
    pub resource_production_per_hour: BTreeMap<String, u64>,
    pub costs: BTreeMap<String, Vec<TerritoryCostTier>>,
    pub ruleset: TerritoryRuleset,
    pub native_map_size: u32,
    #[serde(default)]
    pub supports_horse: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sites: Vec<TerritorySite>,
}

/// The stable subset returned by the catalog list endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerritoryMapSummary {
    pub map: String,
    pub name: String,
    pub season: TerritorySeason,
    pub order: i32,
    pub categories: Vec<String>,
    pub overview_url: String,
}

/// An immutable shared-plan record.
///
/// The identifier is generated by the API. `created_at` is persistence metadata
/// and is not part of the public share response.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerritoryPlanSnapshot {
    pub id: String,
    pub plan: TerritoryPlan,
    pub created_at: DateTime,
}

/// MongoDB access for map catalogs and insert-only plan snapshots.
#[derive(Debug, Clone)]
pub struct TerritoryPlannerV2Store {
    maps: Collection<TerritoryMapCatalog>,
    plans: Collection<TerritoryPlanSnapshot>,
}

impl TerritoryPlannerV2Store {
    /// Opens the v2 catalog and shared-plan collections in `db`.
    pub fn new(db: mongodb::Database) -> Self {
        Self { maps: db.collection(MAPS_COLLECTION), plans: db.collection(PLANS_COLLECTION) }
    }

    /// Creates the unique map and share-ID indexes required by route lookups.
    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        self.maps.create_index(unique_index("map")).await?;
        self.plans.create_index(unique_index("id")).await?;
        Ok(())
    }

    /// Returns at most 64 v2 catalogs in picker order.
    pub async fn list_maps(&self) -> mongodb::error::Result<Vec<TerritoryMapCatalog>> {
        self.maps
            .find(doc! { "schemaVersion": 2 })
            .projection(doc! { "_id": 0 })
            .sort(doc! { "order": 1, "map": 1 })
            .limit(64)
            .await?
            .try_collect()
            .await
    }

    /// Finds a v2 catalog by its validated map identifier.
    pub async fn find_map(&self, map: &str) -> mongodb::error::Result<Option<TerritoryMapCatalog>> {
        self.maps
            .find_one(doc! { "schemaVersion": 2, "map": map })
            .projection(doc! { "_id": 0 })
            .await
    }

    /// Inserts a new snapshot without replacing an existing share.
    pub async fn insert_plan(&self, snapshot: TerritoryPlanSnapshot) -> mongodb::error::Result<()> {
        self.plans.insert_one(snapshot).await?;
        Ok(())
    }

    /// Finds an immutable snapshot by its server-generated identifier.
    pub async fn find_plan(
        &self,
        id: &str,
    ) -> mongodb::error::Result<Option<TerritoryPlanSnapshot>> {
        self.plans.find_one(doc! { "id": id }).projection(doc! { "_id": 0 }).await
    }
}

fn unique_index(field: &str) -> IndexModel {
    IndexModel::builder()
        .keys(doc! { field: 1 })
        .options(IndexOptions::builder().unique(true).build())
        .build()
}
