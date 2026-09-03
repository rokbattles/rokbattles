export type ResourceKind = "food" | "wood" | "stone" | "coin" | "crystal";
export type LandmarkKind = "village" | "cave";
export type MapStructureKind = "pass" | "holy-site" | "ancient-battlefield" | "bastion";
export type BuildingKind = "flag" | "mainFortress" | "subFortress" | "horse";

export type TerritoryMapIndexRow = {
  slug: string;
  title: string;
  order: number;
  ruleset: "home" | "lost-land";
  supportsHorse: boolean;
};

export type TerritoryMapListResponse = { maps: TerritoryMapIndexRow[] };

export type TerritoryApiBuildingKind = "flag" | "centerFortress" | "allianceFortress" | "horse";

export type TerritoryApiBuildingConfig = {
  limit: number;
};

export type CostResourceKind = "credits" | "food" | "wood" | "stone" | "gold" | "crystal";
export type BuildingCost = Partial<Record<CostResourceKind, number>>;

export type BuildingCostTier = {
  from: number;
  to: number;
  cost: BuildingCost;
};

export type BuildingCostSchedule = Partial<Record<BuildingKind, BuildingCostTier[]>>;

export type ImageBounds = {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
};

export type TerritoryMapApiConfig = {
  schemaVersion: number;
  slug: string;
  title: string;
  order: number;
  ruleset: "home" | "lost-land";
  supportsHorse: boolean;
  imageFile: string;
  nativeMapSize: number;
  imageBounds: ImageBounds;
  spatial: {
    chunkSize: number;
    chunkBuffer: number;
    province: boolean;
    chunks: Array<[number, number]>;
  };
  buildings: Partial<Record<TerritoryApiBuildingKind, TerritoryApiBuildingConfig>>;
  resourceProductionPerHour: Record<"food" | "wood" | "stone" | "gold" | "crystal", number>;
  costs: Partial<Record<TerritoryApiBuildingKind, BuildingCostTier[]>>;
};

export type MeshDefinition = {
  id: number;
  name: string;
  vertices: Array<[number, number]>;
  indices: number[];
};

export type MeshInstance = {
  mesh: number;
  affine: [number, number, number, number, number, number];
};

export type ResourcePoint = { id: number; kind: ResourceKind; x: number; y: number };
export type MapLandmark = {
  id: number;
  kind: LandmarkKind;
  x: number;
  y: number;
};

export type MapStructure = {
  id: number;
  strongholdType: number;
  kind: MapStructureKind;
  x: number;
  y: number;
  label: string;
  collision:
    | { shape: "world-square"; halfSize: number }
    | { shape: "territory-square"; radiusInCells: number };
  territoryRadiusInCells: number;
  teleportRadiusInCells: number | null;
  claimable: boolean;
};

export type SpatialChunk = {
  x: number;
  y: number;
  instances: MeshInstance[];
  resources: ResourcePoint[];
  landmarks: MapLandmark[];
  structures: MapStructure[];
};

export type ProvinceRestrictionGrid = {
  cellSize: number;
  width: number;
  height: number;
  serverSchema: number;
  effectiveSchema: number;
  cells: Uint8Array;
  flagBlocked: ReadonlySet<number>;
  fortressBlocked: ReadonlySet<number>;
};

export type Alliance = { id: string; name: string; color: string };

export type DrawingPoint = { x: number; y: number };

export type PlannedDrawing = {
  id: string;
  allianceId: string;
  points: DrawingPoint[];
};

export type PlannedBuilding = {
  id: string;
  allianceId: string;
  kind: BuildingKind;
  x: number;
  y: number;
};

export type PlannerDocument = {
  version: 2;
  mapSlug: string;
  activeAllianceId: string;
  alliances: Alliance[];
  buildings: PlannedBuilding[];
  drawings: PlannedDrawing[];
};

export type PlannerTool = "select" | "draw" | BuildingKind;
