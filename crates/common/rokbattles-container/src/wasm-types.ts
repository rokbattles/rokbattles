export type JsonValue =
  | null
  | boolean
  | number
  | bigint
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };
export type TerritoryResourceKind = "food" | "wood" | "stone" | "coin" | "crystal";
export type TerritoryLandmarkKind = "village" | "cave";
export type TerritoryStructureKind = "pass" | "holy-site" | "ancient-battlefield" | "bastion";
export interface TerritoryMeshDefinition {
  id: number;
  name: string;
  vertices: [number, number][];
  indices: number[];
}
export interface TerritoryMeshInstance {
  mesh: number;
  affine: [number, number, number, number, number, number];
}
export interface TerritoryResourcePoint {
  id: number;
  kind: TerritoryResourceKind;
  x: number;
  y: number;
}
export interface TerritoryLandmark {
  id: number;
  kind: TerritoryLandmarkKind;
  x: number;
  y: number;
}
export interface TerritoryStructure {
  id: number;
  strongholdType: number;
  kind: TerritoryStructureKind;
  x: number;
  y: number;
  label: string;
  collision:
    | { shape: "world-square"; halfSize: number }
    | { shape: "territory-square"; radiusInCells: number };
  territoryRadiusInCells: number;
  teleportRadiusInCells: number | null;
  claimable: boolean;
}
export interface TerritorySpatialChunk {
  x: number;
  y: number;
  instances: TerritoryMeshInstance[];
  resources: TerritoryResourcePoint[];
  landmarks: TerritoryLandmark[];
  structures: TerritoryStructure[];
}
export interface TerritoryProvinceGrid {
  cellSize: number;
  width: number;
  height: number;
  serverSchema: number;
  effectiveSchema: number;
  cells: Uint8Array;
  flagBlocked: number[];
  fortressBlocked: number[];
}
/** Result types selected and checked by the Rust reader. */
export interface SchemaValues {
  1: Uint8Array;
  2: string;
  3: JsonValue;
  401: TerritoryMeshDefinition[];
  402: TerritorySpatialChunk;
  403: TerritoryProvinceGrid;
}
export type SchemaId = keyof SchemaValues;
export type ContainerValue = SchemaValues[SchemaId];
export type SchemaDecoded<S extends SchemaId> = Omit<DecodedValue, "schemaId" | "value"> & {
  readonly schemaId: S;
  readonly value: SchemaValues[S];
};
export interface Reader {
  /** Decodes a file and requires the supplied schema, with its result type inferred. */
  decode<S extends SchemaId>(bytes: Uint8Array, expected_schema: S): SchemaDecoded<S>;
  /** Decodes any supported schema; absent or null expected_schema omits the schema check. */
  decode(bytes: Uint8Array, expected_schema?: number | null): DecodedValue;
}
