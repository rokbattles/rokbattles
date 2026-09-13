//! Public Territory Planner v2 catalog and immutable sharing endpoints.
//!
//! Catalog routes expose bounded rendering metadata and redirect approved binary
//! categories to a fixed CDN origin. Share creation accepts a closed, size-limited
//! schema, validates all references against private catalog sites, and always
//! inserts a new snapshot. There are no update, delete, or account-linkage routes.

use std::{collections::HashSet, sync::Arc};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderValue, StatusCode, header::LOCATION},
    response::IntoResponse,
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use mongodb::{
    bson::DateTime,
    error::{ErrorKind, WriteFailure},
};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::{
    db::{TerritoryMapCatalog, TerritoryMapSummary, TerritoryPlanSnapshot},
    error::ApiError,
    state::AppState,
};

const CDN_BASE: &str = "https://cdn.rokbattles.com/game/territory";
const MAX_BODY_BYTES: usize = 256 * 1024;
const MAX_ALLIANCES: usize = 16;
const MAX_ANNOTATIONS: usize = 500;
const MAX_BUILDINGS: usize = 1_000;
const MAX_ROUTES: usize = 5;
const MAX_ROUTE_SITES: usize = 512;
const MAX_PASSES: usize = 128;
const MAX_ID_BYTES: usize = 64;
const MAX_NAME_BYTES: usize = 80;
const MAX_TEXT_BYTES: usize = 500;
const MAX_TEXT_ANNOTATIONS: usize = 128;
// Revalidate catalog metadata while allowing intermediaries to retain a cached copy.
const CACHE_CONTROL: [(&str, &str); 1] = [("Cache-Control", "public, no-cache")];

/// Builds the versioned catalog, asset redirect, and share routes.
///
/// The body limit is local to share creation; catalog and share reads have no
/// request body.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/list", get(list))
        .route("/map/{map}", get(get_map))
        .route("/map/{map}/bin/{category}", get(get_bin))
        .route("/share", post(share).layer(DefaultBodyLimit::max(MAX_BODY_BYTES)))
        .route("/share/{id}", get(get_share))
}

/// The planner behavior stored in a shared plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerritoryMode {
    Territory,
    Baulur,
}

/// The editable plan stored for a share, excluding derived rendering data.
///
/// Unknown fields are rejected. Camera state, rendered paths, and arbitrary
/// metadata are intentionally absent from this contract.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerritoryPlan {
    pub version: u8,
    pub map: String,
    pub mode: TerritoryMode,
    pub alliances: Vec<PlanAlliance>,
    pub annotations: Vec<PlanAnnotation>,
    pub buildings: Vec<PlanBuilding>,
    pub routes: Vec<PlanRoute>,
}

/// A named alliance referenced by buildings and assigned passes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanAlliance {
    pub id: String,
    pub name: String,
    pub color: String,
}

/// A supported annotation primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnnotationKind {
    Line,
    Arrow,
    Circle,
    Text,
    Marker,
}

/// A bounded annotation in native map coordinates.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanAnnotation {
    pub id: String,
    pub kind: AnnotationKind,
    pub points: Vec<[f64; 2]>,
    pub color: String,
    pub width: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// A planned building associated with one alliance.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanBuilding {
    pub id: String,
    pub kind: String,
    pub alliance_id: String,
    pub x: f64,
    pub y: f64,
}

/// A Baulur route through catalog sites and assigned passes.
///
/// New documents require one `commander` string, which is empty when unassigned.
/// Deserialization also accepts the former two-element `commanders` field so
/// existing immutable snapshots remain readable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanRoute {
    pub id: String,
    pub name: String,
    pub color: String,
    pub commander: String,
    pub site_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_site_id: Option<String>,
    pub passes: Vec<PlanPass>,
}

#[derive(Default)]
struct Present<T>(Option<T>);

// Presence is tracked separately from the value so null and absent fields cannot
// collapse into the same `Option` state during compatibility deserialization.
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Present<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(|value| Self(Some(value)))
    }
}

// This wire type keeps legacy input handling out of the current serialized shape.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlanRouteWire {
    id: String,
    name: String,
    color: String,
    #[serde(default)]
    commander: Present<String>,
    #[serde(default)]
    commanders: Present<[String; 2]>,
    site_ids: Vec<String>,
    #[serde(default)]
    start_site_id: Option<String>,
    passes: Vec<PlanPass>,
}

impl<'de> Deserialize<'de> for PlanRoute {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = PlanRouteWire::deserialize(deserializer)?;
        let commander = match (wire.commander.0, wire.commanders.0) {
            (Some(commander), None) => commander,
            (None, Some([primary, secondary])) => {
                if primary.is_empty() {
                    secondary
                } else {
                    primary
                }
            }
            (Some(_), Some(_)) => {
                return Err(serde::de::Error::custom("commander fields are mutually exclusive"));
            }
            (None, None) => return Err(serde::de::Error::missing_field("commander")),
        };
        Ok(Self {
            id: wire.id,
            name: wire.name,
            color: wire.color,
            commander,
            site_ids: wire.site_ids,
            start_site_id: wire.start_site_id,
            passes: wire.passes,
        })
    }
}

/// A pass reference with an alliance assignment.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanPass {
    pub site_id: String,
    pub alliance_id: String,
}

#[derive(Serialize)]
struct ListResponse {
    maps: Vec<TerritoryMapSummary>,
}

#[derive(Serialize)]
struct ShareCreated {
    id: String,
}

#[derive(Serialize)]
struct ShareResponse {
    id: String,
    plan: TerritoryPlan,
}

// List responses contain picker metadata only; private site records never enter
// the response shape.
async fn list(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, ApiError> {
    let maps = state.territory_planner_v2.list_maps().await.map_err(internal_mongo)?;
    let maps = maps
        .into_iter()
        .map(|map| TerritoryMapSummary {
            overview_url: format!("{CDN_BASE}/{}/overview.webp", map.map),
            map: map.map,
            name: map.name,
            season: map.season,
            order: map.order,
            categories: map.categories,
        })
        .collect();
    Ok((StatusCode::OK, CACHE_CONTROL, Json(ListResponse { maps })))
}

// The stored catalog includes private validation sites. Clear them before serde
// observes the value so the public contract contains metadata only.
async fn get_map(
    State(state): State<Arc<AppState>>,
    Path(map): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    validate_token(&map, "map")?;
    let mut map = load_map(&state, &map).await?;
    map.sites.clear();
    Ok((StatusCode::OK, CACHE_CONTROL, Json(map)))
}

// Redirect only categories declared by the catalog. The origin is constant, and
// both path components are restricted tokens rather than caller-provided URLs.
async fn get_bin(
    State(state): State<Arc<AppState>>,
    Path((map, category)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    validate_token(&map, "map")?;
    validate_token(&category, "category")?;
    let catalog = load_map(&state, &map).await?;
    let filename = format!("{category}.bin");
    if !catalog.categories.iter().any(|candidate| candidate == &category)
        || !catalog.assets.contains_key(&filename)
    {
        return Err(ApiError::not_found("territory planner asset not found"));
    }
    let location =
        HeaderValue::from_str(&format!("{CDN_BASE}/{map}/{filename}")).map_err(|header_error| {
            error!(error = %header_error, "catalog produced an invalid asset location");
            ApiError::internal("invalid catalog asset location")
        })?;
    Ok((StatusCode::TEMPORARY_REDIRECT, [(LOCATION, location)]))
}

// Validate the plan before assigning a cryptographically random share ID.
// Duplicate keys are retried with a fresh ID, never replaced.
async fn share(
    State(state): State<Arc<AppState>>,
    Json(plan): Json<TerritoryPlan>,
) -> Result<impl IntoResponse, ApiError> {
    validate_token(&plan.map, "map")?;
    let catalog = load_map(&state, &plan.map).await?;
    validate_plan(&plan, &catalog)?;

    for _ in 0..3 {
        let id = random_id();
        let snapshot = TerritoryPlanSnapshot {
            id: id.clone(),
            plan: plan.clone(),
            created_at: DateTime::now(),
        };
        match state.territory_planner_v2.insert_plan(snapshot).await {
            Ok(()) => return Ok((StatusCode::CREATED, Json(ShareCreated { id }))),
            Err(error) if is_duplicate_key(&error) => continue,
            Err(error) => return Err(internal_mongo(error)),
        }
    }
    Err(ApiError::internal("could not allocate plan identifier"))
}

// Reads validate both the share ID and the stored plan. Invalid,
// corrupt, and missing snapshots deliberately share the same not-found response.
async fn get_share(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    if !is_share_id(&id) {
        return Err(ApiError::not_found("territory plan not found"));
    }
    let snapshot = state
        .territory_planner_v2
        .find_plan(&id)
        .await
        .map_err(internal_mongo)?
        .ok_or_else(|| ApiError::not_found("territory plan not found"))?;
    if validate_token(&snapshot.plan.map, "map").is_err() {
        error!(plan_id = %snapshot.id, "stored territory plan has an invalid map identifier");
        return Err(ApiError::not_found("territory plan not found"));
    }
    let Some(catalog) =
        state.territory_planner_v2.find_map(&snapshot.plan.map).await.map_err(internal_mongo)?
    else {
        error!(plan_id = %snapshot.id, "stored territory plan references a missing map");
        return Err(ApiError::not_found("territory plan not found"));
    };
    if validate_stored_plan(&snapshot.plan, &catalog).is_err() {
        error!(plan_id = %snapshot.id, "stored territory plan failed current validation");
        return Err(ApiError::not_found("territory plan not found"));
    }
    Ok((StatusCode::OK, Json(ShareResponse { id: snapshot.id, plan: snapshot.plan })))
}

async fn load_map(state: &AppState, map: &str) -> Result<TerritoryMapCatalog, ApiError> {
    state
        .territory_planner_v2
        .find_map(map)
        .await
        .map_err(internal_mongo)?
        .ok_or_else(|| ApiError::not_found("territory planner map not found"))
}

fn validate_plan(plan: &TerritoryPlan, catalog: &TerritoryMapCatalog) -> Result<(), ApiError> {
    validate_plan_with_alliance_names(plan, catalog, validate_alliance_name)
}

// Stored snapshots use the former alliance-name bound on reads. New writes use
// the shorter tag limit, but that later UI constraint must not invalidate old URLs.
fn validate_stored_plan(
    plan: &TerritoryPlan,
    catalog: &TerritoryMapCatalog,
) -> Result<(), ApiError> {
    validate_plan_with_alliance_names(plan, catalog, validate_name)
}

// Apply count limits before building lookup tables, then validate every foreign
// key against catalog metadata. This keeps allocation and cross-reference work
// bounded for both requests and persisted documents.
fn validate_plan_with_alliance_names(
    plan: &TerritoryPlan,
    catalog: &TerritoryMapCatalog,
    validate_alliance_name: fn(&str) -> Result<(), ApiError>,
) -> Result<(), ApiError> {
    if plan.version != 2 || plan.map != catalog.map {
        return invalid_plan();
    }
    match plan.mode {
        TerritoryMode::Territory if !plan.routes.is_empty() => return invalid_plan(),
        TerritoryMode::Baulur
            if catalog.ruleset != crate::db::TerritoryRuleset::Home
                || catalog.season != crate::db::TerritorySeason::Preparation
                || !catalog.categories.iter().any(|category| category == "canyons") =>
        {
            return invalid_plan();
        }
        TerritoryMode::Territory | TerritoryMode::Baulur => {}
    }
    bounded_len(&plan.alliances, MAX_ALLIANCES)?;
    bounded_len(&plan.annotations, MAX_ANNOTATIONS)?;
    bounded_len(&plan.buildings, MAX_BUILDINGS)?;
    bounded_len(&plan.routes, MAX_ROUTES)?;
    if plan.alliances.is_empty() {
        return invalid_plan();
    }

    let mut object_ids = HashSet::new();
    let mut alliance_ids = HashSet::new();
    let mut text_annotations = 0;
    for alliance in &plan.alliances {
        validate_id(&alliance.id)?;
        validate_alliance_name(&alliance.name)?;
        validate_color(&alliance.color)?;
        if !alliance_ids.insert(alliance.id.as_str()) || !object_ids.insert(alliance.id.as_str()) {
            return invalid_plan();
        }
    }

    for annotation in &plan.annotations {
        validate_object_id(&annotation.id, &mut object_ids)?;
        validate_color(&annotation.color)?;
        let valid_cardinality = match annotation.kind {
            AnnotationKind::Line => (2..=256).contains(&annotation.points.len()),
            AnnotationKind::Arrow | AnnotationKind::Circle => annotation.points.len() == 2,
            AnnotationKind::Text | AnnotationKind::Marker => annotation.points.len() == 1,
        };
        if !(1..=3).contains(&annotation.width) || !valid_cardinality {
            return invalid_plan();
        }
        for point in &annotation.points {
            validate_point(*point, catalog.bounds)?;
        }
        match (&annotation.kind, &annotation.text) {
            (AnnotationKind::Text, Some(text))
                if !text.trim().is_empty()
                    && text.len() <= MAX_TEXT_BYTES
                    && !text.chars().any(char::is_control) =>
            {
                text_annotations += 1;
                if text_annotations > MAX_TEXT_ANNOTATIONS {
                    return invalid_plan();
                }
            }
            (AnnotationKind::Text, _) | (_, Some(_)) => return invalid_plan(),
            (_, None) => {}
        }
    }

    let mut building_counts = std::collections::HashMap::new();
    for building in &plan.buildings {
        validate_object_id(&building.id, &mut object_ids)?;
        if building.kind == "horse" && !catalog.supports_horse {
            return invalid_plan();
        }
        let Some(rule) = catalog.buildings.get(&building.kind) else { return invalid_plan() };
        if !alliance_ids.contains(building.alliance_id.as_str()) {
            return invalid_plan();
        }
        let count = building_counts
            .entry((building.alliance_id.as_str(), building.kind.as_str()))
            .or_insert(0_u32);
        *count += 1;
        if *count > rule.limit {
            return invalid_plan();
        }
        validate_point([building.x, building.y], catalog.bounds)?;
    }

    let sites: std::collections::HashMap<_, _> =
        catalog.sites.iter().map(|site| (site.id.as_str(), site)).collect();
    let mut assigned_sites = HashSet::new();
    let mut assigned_passes = HashSet::new();
    for route in &plan.routes {
        validate_object_id(&route.id, &mut object_ids)?;
        validate_name(&route.name)?;
        validate_color(&route.color)?;
        if !route.commander.is_empty() {
            validate_name(&route.commander)?;
        }
        bounded_len(&route.site_ids, MAX_ROUTE_SITES)?;
        bounded_len(&route.passes, MAX_PASSES)?;
        if route.site_ids.iter().any(|id| {
            !id.starts_with("canyons:")
                || !sites.contains_key(id.as_str())
                || !assigned_sites.insert(id.as_str())
        }) {
            return invalid_plan();
        }
        if route.start_site_id.as_ref().is_some_and(|id| !route.site_ids.contains(id)) {
            return invalid_plan();
        }
        for pass in &route.passes {
            let Some(site) = sites.get(pass.site_id.as_str()) else { return invalid_plan() };
            if site.kind != "pass"
                || !alliance_ids.contains(pass.alliance_id.as_str())
                || !assigned_passes.insert(pass.site_id.as_str())
            {
                return invalid_plan();
            }
        }
    }
    Ok(())
}

fn validate_point(point: [f64; 2], bounds: [f64; 4]) -> Result<(), ApiError> {
    if point.into_iter().chain(bounds).any(|value| !value.is_finite())
        || point[0] < bounds[0]
        || point[0] > bounds[2]
        || point[1] < bounds[1]
        || point[1] > bounds[3]
    {
        return invalid_plan();
    }
    Ok(())
}

fn validate_object_id<'a>(id: &'a str, ids: &mut HashSet<&'a str>) -> Result<(), ApiError> {
    validate_id(id)?;
    if !ids.insert(id) {
        return invalid_plan();
    }
    Ok(())
}

fn validate_id(value: &str) -> Result<(), ApiError> {
    if value.is_empty() || value.len() > MAX_ID_BYTES || !value.bytes().all(is_identifier_byte) {
        return invalid_plan();
    }
    Ok(())
}

fn validate_name(value: &str) -> Result<(), ApiError> {
    if value.is_empty() || value.len() > MAX_NAME_BYTES || value.chars().any(char::is_control) {
        return invalid_plan();
    }
    Ok(())
}

fn validate_alliance_name(value: &str) -> Result<(), ApiError> {
    if value.trim().is_empty() || value.chars().count() > 4 || value.chars().any(char::is_control) {
        return invalid_plan();
    }
    Ok(())
}

fn validate_color(value: &str) -> Result<(), ApiError> {
    if value.len() != 7
        || !value.starts_with('#')
        || !value.bytes().skip(1).all(|byte| byte.is_ascii_hexdigit())
    {
        return invalid_plan();
    }
    Ok(())
}

fn validate_token(value: &str, field: &str) -> Result<(), ApiError> {
    if value.is_empty() || value.len() > 64 || !value.bytes().all(is_identifier_byte) {
        return Err(ApiError::bad_request(format!("invalid {field}")));
    }
    Ok(())
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':')
}

fn bounded_len<T>(values: &[T], maximum: usize) -> Result<(), ApiError> {
    if values.len() > maximum {
        return invalid_plan();
    }
    Ok(())
}

fn invalid_plan<T>() -> Result<T, ApiError> {
    Err(ApiError::bad_request("invalid territory plan"))
}

fn random_id() -> String {
    let mut bytes = [0_u8; 24];
    rand::rng().fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn is_share_id(id: &str) -> bool {
    id.len() == 32
        && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn is_duplicate_key(error: &mongodb::error::Error) -> bool {
    matches!(
        error.kind.as_ref(),
        ErrorKind::Write(WriteFailure::WriteError(write_error)) if write_error.code == 11000
    )
}

fn internal_mongo(error_value: mongodb::error::Error) -> ApiError {
    error!(error = %error_value, "territory planner database operation failed");
    ApiError::internal("territory planner database operation failed")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::db::{TerritoryBuildingRule, TerritoryRuleset, TerritorySeason, TerritorySite};

    fn catalog() -> TerritoryMapCatalog {
        TerritoryMapCatalog {
            schema_version: 2,
            map: "ark".into(),
            name: "Ark".into(),
            season: TerritorySeason::Preparation,
            order: 1,
            bounds: [0.0, 0.0, 100.0, 100.0],
            pixel_size: [1024, 1024],
            categories: vec!["canyons".into()],
            assets: BTreeMap::new(),
            structure_icons: BTreeMap::new(),
            building_icons: BTreeMap::new(),
            site_icons: BTreeMap::new(),
            buildings: BTreeMap::from([("flag".into(), TerritoryBuildingRule { limit: 500 })]),
            structure_rules: BTreeMap::new(),
            resource_production_per_hour: BTreeMap::new(),
            costs: BTreeMap::new(),
            ruleset: TerritoryRuleset::Home,
            native_map_size: 100,
            supports_horse: false,
            sites: vec![TerritorySite {
                id: "canyons:1".into(),
                kind: "camp".into(),
                x: 10.0,
                y: 10.0,
            }],
        }
    }

    fn plan() -> TerritoryPlan {
        TerritoryPlan {
            version: 2,
            map: "ark".into(),
            mode: TerritoryMode::Territory,
            alliances: vec![PlanAlliance {
                id: "a1".into(),
                name: "ROKB".into(),
                color: "#aabbcc".into(),
            }],
            annotations: vec![],
            buildings: vec![],
            routes: vec![],
        }
    }

    #[test]
    fn validation_should_accept_minimal_plan() {
        validate_plan(&plan(), &catalog()).expect("minimal plan should validate");
    }

    #[test]
    fn validation_should_bound_alliance_names_by_unicode_scalar_count() {
        let mut value = plan();
        value.alliances[0].name = "王国🛡A".into();
        validate_plan(&value, &catalog()).expect("four or fewer Unicode scalars should validate");

        value.alliances[0].name = "ABCDE".into();
        assert!(validate_plan(&value, &catalog()).is_err());
        value.alliances[0].name = " \t ".into();
        assert!(validate_plan(&value, &catalog()).is_err());
    }

    #[test]
    fn stored_plan_validation_should_accept_legacy_alliance_names() {
        let mut value = plan();
        value.alliances[0].name = "Legacy Alliance".into();
        assert!(validate_plan(&value, &catalog()).is_err());
        validate_stored_plan(&value, &catalog())
            .expect("a previously valid immutable snapshot should remain readable");
    }

    #[test]
    fn validation_should_reject_unknown_fields() {
        let json = r##"{"version":2,"map":"ark","mode":"territory","alliances":[],"annotations":[],"buildings":[],"routes":[],"metadata":{"url":"http://127.0.0.1"}}"##;
        serde_json::from_str::<TerritoryPlan>(json).expect_err("metadata should be rejected");
    }

    #[test]
    fn validation_should_reject_non_finite_or_out_of_bounds_points() {
        let mut value = plan();
        value.annotations.push(PlanAnnotation {
            id: "n1".into(),
            kind: AnnotationKind::Marker,
            points: vec![[101.0, 5.0]],
            color: "#ffffff".into(),
            width: 1,
            text: None,
        });
        assert!(validate_plan(&value, &catalog()).is_err());
    }

    #[test]
    fn validation_should_enforce_annotation_cardinality_and_text_controls() {
        let mut value = plan();
        value.annotations.push(PlanAnnotation {
            id: "n1".into(),
            kind: AnnotationKind::Text,
            points: vec![[10.0, 10.0], [20.0, 20.0]],
            color: "#ffffff".into(),
            width: 1,
            text: Some("unsafe\nlabel".into()),
        });
        assert!(validate_plan(&value, &catalog()).is_err());
    }

    #[test]
    fn validation_should_reject_sites_assigned_to_multiple_routes() {
        let mut value = plan();
        value.mode = TerritoryMode::Baulur;
        let route = PlanRoute {
            id: "r1".into(),
            name: "Route".into(),
            color: "#ffffff".into(),
            commander: "One".into(),
            site_ids: vec!["canyons:1".into()],
            start_site_id: Some("canyons:1".into()),
            passes: vec![],
        };
        value.routes = vec![route.clone(), PlanRoute { id: "r2".into(), ..route }];
        assert!(validate_plan(&value, &catalog()).is_err());
    }

    #[test]
    fn validation_should_allow_an_optional_baulur_commander() {
        let mut value = plan();
        value.mode = TerritoryMode::Baulur;
        value.routes.push(PlanRoute {
            id: "r1".into(),
            name: "Route".into(),
            color: "#ffffff".into(),
            commander: String::new(),
            site_ids: vec!["canyons:1".into()],
            start_site_id: None,
            passes: vec![],
        });
        validate_plan(&value, &catalog()).expect("empty commander slots should be optional");

        value.routes[0].commander = "Primary".into();
        validate_plan(&value, &catalog()).expect("one commander should be accepted");

        value.routes[0].commander = "unsafe\nname".into();
        assert!(validate_plan(&value, &catalog()).is_err());
        value.routes[0].commander = "x".repeat(MAX_NAME_BYTES + 1);
        assert!(validate_plan(&value, &catalog()).is_err());
    }

    #[test]
    fn route_wire_format_should_normalize_legacy_commander_slots() {
        let legacy = r##"{"id":"r1","name":"Route","color":"#ffffff","commanders":["","Secondary"],"siteIds":[],"passes":[]}"##;
        let route: PlanRoute =
            serde_json::from_str(legacy).expect("legacy route should deserialize");
        assert_eq!(route.commander, "Secondary");
        let serialized = serde_json::to_value(&route).expect("route should serialize");
        assert_eq!(serialized["commander"], "Secondary");
        assert!(serialized.get("commanders").is_none());
    }

    #[test]
    fn route_wire_format_should_reject_ambiguous_or_malformed_commander_fields() {
        for json in [
            r##"{"id":"r1","name":"Route","color":"#ffffff","commander":"One","commanders":["One","Two"],"siteIds":[],"passes":[]}"##,
            r##"{"id":"r1","name":"Route","color":"#ffffff","commander":null,"siteIds":[],"passes":[]}"##,
            r##"{"id":"r1","name":"Route","color":"#ffffff","commanders":["One"],"siteIds":[],"passes":[]}"##,
        ] {
            serde_json::from_str::<PlanRoute>(json).expect_err("invalid commander wire form");
        }
    }

    #[test]
    fn validation_should_bound_route_passes_to_navigation_capacity() {
        let mut value = plan();
        value.mode = TerritoryMode::Baulur;
        value.routes.push(PlanRoute {
            id: "r1".into(),
            name: "Route".into(),
            color: "#ffffff".into(),
            commander: "One".into(),
            site_ids: vec![],
            start_site_id: None,
            passes: (0..=MAX_PASSES)
                .map(|_| PlanPass { site_id: "canyons:1".into(), alliance_id: "a1".into() })
                .collect(),
        });
        assert!(validate_plan(&value, &catalog()).is_err());
    }

    #[test]
    fn validation_should_enforce_building_limits_per_alliance() {
        let mut value = plan();
        value.buildings = (0..501)
            .map(|index| PlanBuilding {
                id: format!("b{index}"),
                kind: "flag".into(),
                alliance_id: "a1".into(),
                x: 10.0,
                y: 10.0,
            })
            .collect();
        assert!(validate_plan(&value, &catalog()).is_err());
    }

    #[test]
    fn validation_should_allow_one_horse_only_on_supported_maps() {
        let horse = PlanBuilding {
            id: "horse-1".into(),
            kind: "horse".into(),
            alliance_id: "a1".into(),
            x: 10.0,
            y: 10.0,
        };
        let mut supported = catalog();
        supported.supports_horse = true;
        supported.buildings.insert("horse".into(), TerritoryBuildingRule { limit: 1 });
        let mut value = plan();
        value.buildings.push(horse.clone());
        validate_plan(&value, &supported).expect("supported map should allow one horse");

        let mut unsupported = catalog();
        unsupported.buildings.insert("horse".into(), TerritoryBuildingRule { limit: 1 });
        assert!(validate_plan(&value, &unsupported).is_err());
        value.buildings.push(PlanBuilding { id: "horse-2".into(), ..horse });
        assert!(validate_plan(&value, &supported).is_err());
    }

    #[test]
    fn share_ids_should_be_url_safe_and_unguessable_length() {
        let id = random_id();
        assert!(is_share_id(&id));
        assert_ne!(id, random_id());
    }
}
