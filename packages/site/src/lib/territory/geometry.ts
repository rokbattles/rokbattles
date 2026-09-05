import type {
  BuildingKind,
  MapStructure,
  MeshDefinition,
  MeshInstance,
  PlannedBuilding,
  ProvinceRestrictionGrid,
  ResourceKind,
  ResourcePoint,
} from "./types";

export const TERRITORY_UNIT = 18;
export const BUILD_POSITION_ALIGNMENT = 0.01;

export type TerritoryFillRect = {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
};

export type TerritoryBoundarySegment = {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
};

export type TerritoryOverview = {
  fillRects: TerritoryFillRect[];
  boundarySegments: TerritoryBoundarySegment[];
};

export type TerritoryCell = {
  column: number;
  row: number;
};

export type TerritoryCellOwner = TerritoryCell & {
  allianceId: string;
};

export type TerritoryOwnership = ReadonlyMap<string, TerritoryCellOwner>;
export type TerritoryState = {
  ownership: TerritoryOwnership;
  structureOwners: ReadonlyMap<number, string>;
};

export const buildingRules: Record<
  BuildingKind,
  { territorySide: number; baseClearance: number; root: boolean }
> = {
  flag: { territorySide: 54, baseClearance: 1.4, root: false },
  mainFortress: {
    territorySide: 90,
    baseClearance: 3.75,
    root: true,
  },
  subFortress: {
    territorySide: 90,
    baseClearance: 3,
    root: true,
  },
  horse: { territorySide: 54, baseClearance: 3, root: true },
};

export function isBuildingKindAvailable(
  kind: BuildingKind,
  ruleset: "home" | "lost-land",
  supportsHorse: boolean
): boolean {
  if (kind === "mainFortress") return ruleset === "home";
  if (kind === "horse") return supportsHorse;
  return true;
}

export function snapTerritoryCoordinate(value: number): number {
  return Math.floor(value / TERRITORY_UNIT) * TERRITORY_UNIT + TERRITORY_UNIT / 2;
}

export function snapTerritoryPoint(x: number, y: number): { x: number; y: number } {
  return { x: snapTerritoryCoordinate(x), y: snapTerritoryCoordinate(y) };
}

export function alignBuildingCoordinate(value: number): number {
  const aligned = Math.floor(value / BUILD_POSITION_ALIGNMENT + 0.5) * BUILD_POSITION_ALIGNMENT;
  return Number(aligned.toFixed(2));
}

export function alignBuildingPoint(x: number, y: number): { x: number; y: number } {
  return { x: alignBuildingCoordinate(x), y: alignBuildingCoordinate(y) };
}

export function provinceIdAt(
  grid: ProvinceRestrictionGrid | null,
  x: number,
  y: number
): number | null {
  if (!grid || x < 0 || y < 0) return null;
  const column = Math.floor(x / grid.cellSize);
  const row = Math.floor(y / grid.cellSize);
  if (column >= grid.width || row >= grid.height) return null;
  return grid.cells[row * grid.width + column] ?? null;
}

export function isProvinceRestricted(
  grid: ProvinceRestrictionGrid | null,
  kind: BuildingKind,
  x: number,
  y: number
): boolean {
  const provinceId = provinceIdAt(grid, x, y);
  if (provinceId === null) return false;
  return kind === "flag"
    ? grid?.flagBlocked.has(provinceId) === true
    : grid?.fortressBlocked.has(provinceId) === true;
}

export function territoryBounds(building: Pick<PlannedBuilding, "kind" | "x" | "y">) {
  const center = snapTerritoryPoint(building.x, building.y);
  const half = buildingRules[building.kind].territorySide / 2;
  return {
    minX: center.x - half,
    minY: center.y - half,
    maxX: center.x + half,
    maxY: center.y + half,
  };
}

function mergeIntervals(intervals: Array<[number, number]>): Array<[number, number]> {
  intervals.sort((left, right) => left[0] - right[0]);
  const merged: Array<[number, number]> = [];
  for (const interval of intervals) {
    const previous = merged.at(-1);
    if (previous && interval[0] <= previous[1]) previous[1] = Math.max(previous[1], interval[1]);
    else merged.push([...interval]);
  }
  return merged;
}

function territoryCellKey(column: number, row: number): string {
  return `${column}:${row}`;
}

export function structureTerritoryCells(
  structure: Pick<MapStructure, "x" | "y" | "territoryRadiusInCells">
): TerritoryCell[] {
  const centerColumn = Math.floor(structure.x / TERRITORY_UNIT);
  const centerRow = Math.floor(structure.y / TERRITORY_UNIT);
  const cells: TerritoryCell[] = [];
  for (
    let row = centerRow - structure.territoryRadiusInCells;
    row <= centerRow + structure.territoryRadiusInCells;
    row += 1
  ) {
    for (
      let column = centerColumn - structure.territoryRadiusInCells;
      column <= centerColumn + structure.territoryRadiusInCells;
      column += 1
    ) {
      cells.push({ column, row });
    }
  }
  return cells;
}

export function territoryCells(
  building: Pick<PlannedBuilding, "kind" | "x" | "y">
): TerritoryCell[] {
  const centerColumn = Math.floor(building.x / TERRITORY_UNIT);
  const centerRow = Math.floor(building.y / TERRITORY_UNIT);
  const radius = (buildingRules[building.kind].territorySide / TERRITORY_UNIT - 1) / 2;
  const cells: TerritoryCell[] = [];
  for (let row = centerRow - radius; row <= centerRow + radius; row += 1) {
    for (let column = centerColumn - radius; column <= centerColumn + radius; column += 1) {
      cells.push({ column, row });
    }
  }
  return cells;
}

function cellsTouchAlliance(
  cells: TerritoryCell[],
  allianceId: string,
  ownership: TerritoryOwnership
): boolean {
  return cells.some((cell) =>
    [
      [cell.column, cell.row],
      [cell.column - 1, cell.row],
      [cell.column + 1, cell.row],
      [cell.column, cell.row - 1],
      [cell.column, cell.row + 1],
    ].some(
      ([column, row]) => ownership.get(territoryCellKey(column, row))?.allianceId === allianceId
    )
  );
}

export function buildTerritoryState(
  buildings: PlannedBuilding[],
  structures: readonly MapStructure[] = []
): TerritoryState {
  const ownership = new Map<string, TerritoryCellOwner>();
  const structureOwners = new Map<number, string>();
  const structureCellsById = new Map<number, TerritoryCell[]>();
  const reservedStructureCells = new Set<string>();
  const permanentlyExcludedCells = new Set<string>();
  for (const structure of structures) {
    if (structure.territoryRadiusInCells <= 0) continue;
    const cells = structureTerritoryCells(structure);
    structureCellsById.set(structure.id, cells);
    for (const cell of cells) {
      const key = territoryCellKey(cell.column, cell.row);
      reservedStructureCells.add(key);
      if (!structure.claimable) permanentlyExcludedCells.add(key);
    }
  }
  const claimableStructures = structures
    .filter((structure) => structure.claimable && structure.territoryRadiusInCells > 0)
    .toSorted((left, right) => left.id - right.id);
  for (const building of buildings) {
    for (const cell of territoryCells(building)) {
      const key = territoryCellKey(cell.column, cell.row);
      if (reservedStructureCells.has(key) || ownership.has(key)) continue;
      ownership.set(key, {
        ...cell,
        allianceId: building.allianceId,
      });
    }
    let claimedInPass = true;
    while (claimedInPass) {
      claimedInPass = false;
      for (const structure of claimableStructures) {
        if (structureOwners.has(structure.id)) continue;
        const cells = structureCellsById.get(structure.id) ?? [];
        if (!cellsTouchAlliance(cells, building.allianceId, ownership)) continue;
        structureOwners.set(structure.id, building.allianceId);
        for (const cell of cells) {
          const key = territoryCellKey(cell.column, cell.row);
          if (permanentlyExcludedCells.has(key)) continue;
          if (!ownership.has(key)) ownership.set(key, { ...cell, allianceId: building.allianceId });
        }
        claimedInPass = true;
      }
    }
  }
  return { ownership, structureOwners };
}

export function buildTerritoryOwnership(
  buildings: PlannedBuilding[],
  structures: readonly MapStructure[] = []
): TerritoryOwnership {
  return buildTerritoryState(buildings, structures).ownership;
}

export function buildTerritoryOverviewFromCells(
  source: Iterable<TerritoryCell>
): TerritoryOverview {
  const cells = new Map<string, { column: number; row: number }>();
  for (const cell of source) {
    cells.set(territoryCellKey(cell.column, cell.row), cell);
  }

  const columnsByRow = new Map<number, number[]>();
  for (const { column, row } of cells.values()) {
    const columns = columnsByRow.get(row);
    if (columns) columns.push(column);
    else columnsByRow.set(row, [column]);
  }

  const fillRects: TerritoryFillRect[] = [];
  let activeRects = new Map<string, TerritoryFillRect>();
  for (const row of [...columnsByRow.keys()].sort((left, right) => left - right)) {
    const columns = columnsByRow.get(row) ?? [];
    columns.sort((left, right) => left - right);
    const spans: Array<[number, number]> = [];
    for (const column of columns) {
      const previous = spans.at(-1);
      if (previous && column === previous[1]) previous[1] = column + 1;
      else spans.push([column, column + 1]);
    }
    const nextActiveRects = new Map<string, TerritoryFillRect>();
    for (const [minColumn, maxColumn] of spans) {
      const key = `${minColumn}:${maxColumn}`;
      const minY = row * TERRITORY_UNIT;
      const active = activeRects.get(key);
      if (active?.maxY === minY) {
        active.maxY += TERRITORY_UNIT;
        nextActiveRects.set(key, active);
      } else {
        const rectangle = {
          minX: minColumn * TERRITORY_UNIT,
          minY,
          maxX: maxColumn * TERRITORY_UNIT,
          maxY: minY + TERRITORY_UNIT,
        };
        fillRects.push(rectangle);
        nextActiveRects.set(key, rectangle);
      }
    }
    activeRects = nextActiveRects;
  }

  const horizontalIntervals = new Map<number, Array<[number, number]>>();
  const verticalIntervals = new Map<number, Array<[number, number]>>();
  const addInterval = (
    groups: Map<number, Array<[number, number]>>,
    axis: number,
    start: number,
    end: number
  ) => {
    const intervals = groups.get(axis);
    if (intervals) intervals.push([start, end]);
    else groups.set(axis, [[start, end]]);
  };
  for (const { column, row } of cells.values()) {
    const minX = column * TERRITORY_UNIT;
    const minY = row * TERRITORY_UNIT;
    const maxX = minX + TERRITORY_UNIT;
    const maxY = minY + TERRITORY_UNIT;
    if (!cells.has(`${column}:${row - 1}`)) addInterval(horizontalIntervals, minY, minX, maxX);
    if (!cells.has(`${column}:${row + 1}`)) addInterval(horizontalIntervals, maxY, minX, maxX);
    if (!cells.has(`${column - 1}:${row}`)) addInterval(verticalIntervals, minX, minY, maxY);
    if (!cells.has(`${column + 1}:${row}`)) addInterval(verticalIntervals, maxX, minY, maxY);
  }

  const boundarySegments: TerritoryBoundarySegment[] = [];
  for (const y of [...horizontalIntervals.keys()].sort((left, right) => left - right)) {
    for (const [x1, x2] of mergeIntervals(horizontalIntervals.get(y) ?? [])) {
      boundarySegments.push({ x1, y1: y, x2, y2: y });
    }
  }
  for (const x of [...verticalIntervals.keys()].sort((left, right) => left - right)) {
    for (const [y1, y2] of mergeIntervals(verticalIntervals.get(x) ?? [])) {
      boundarySegments.push({ x1: x, y1, x2: x, y2 });
    }
  }
  return { fillRects, boundarySegments };
}

export function buildTerritoryOverview(
  buildings: Array<Pick<PlannedBuilding, "kind" | "x" | "y">>
): TerritoryOverview {
  return buildTerritoryOverviewFromCells(buildings.flatMap(territoryCells));
}

export function buildTerritoryOverviewsByAlliance(
  buildings: PlannedBuilding[],
  structures: readonly MapStructure[] = []
): Map<string, TerritoryOverview> {
  return buildTerritoryOverviewsFromOwnership(buildTerritoryOwnership(buildings, structures));
}

export function buildTerritoryOverviewsFromOwnership(
  ownership: TerritoryOwnership
): Map<string, TerritoryOverview> {
  const cellsByAlliance = new Map<string, TerritoryCell[]>();
  for (const owner of ownership.values()) {
    const cells = cellsByAlliance.get(owner.allianceId);
    if (cells) cells.push(owner);
    else cellsByAlliance.set(owner.allianceId, [owner]);
  }
  return new Map(
    [...cellsByAlliance].map(([allianceId, cells]) => [
      allianceId,
      buildTerritoryOverviewFromCells(cells),
    ])
  );
}

export function isConnectedToAlliance(
  candidate: Pick<PlannedBuilding, "kind" | "x" | "y" | "allianceId">,
  ownership: TerritoryOwnership
): boolean {
  if (buildingRules[candidate.kind].root) return true;
  return territoryCells(candidate).some((cell) =>
    [
      [cell.column, cell.row],
      [cell.column - 1, cell.row],
      [cell.column + 1, cell.row],
      [cell.column, cell.row - 1],
      [cell.column, cell.row + 1],
    ].some(
      ([column, row]) =>
        ownership.get(territoryCellKey(column, row))?.allianceId === candidate.allianceId
    )
  );
}

export function hasRequiredTerritoryAvailability(
  candidate: Pick<PlannedBuilding, "kind" | "x" | "y" | "allianceId">,
  ownership: TerritoryOwnership
): boolean {
  const cells = territoryCells(candidate);
  const sideInCells = Math.round(buildingRules[candidate.kind].territorySide / TERRITORY_UNIT);
  const requiredAvailable = Math.ceil(sideInCells / 2) * sideInCells;
  let available = 0;
  for (const cell of cells) {
    const owner = ownership.get(territoryCellKey(cell.column, cell.row));
    if (!owner || owner.allianceId === candidate.allianceId) available += 1;
  }
  return available >= requiredAvailable;
}

export function plannedBuildingCollision(
  candidate: Pick<PlannedBuilding, "kind" | "x" | "y">,
  buildings: readonly Pick<PlannedBuilding, "kind" | "x" | "y">[]
): boolean {
  const candidateRadius = buildingRules[candidate.kind].baseClearance;
  return buildings.some((building) => {
    const minimumDistance = candidateRadius + buildingRules[building.kind].baseClearance;
    return Math.hypot(candidate.x - building.x, candidate.y - building.y) < minimumDistance;
  });
}

function circleIntersectsRectangle(
  x: number,
  y: number,
  radius: number,
  minX: number,
  minY: number,
  maxX: number,
  maxY: number
): boolean {
  const nearestX = Math.max(minX, Math.min(maxX, x));
  const nearestY = Math.max(minY, Math.min(maxY, y));
  return (x - nearestX) ** 2 + (y - nearestY) ** 2 <= radius ** 2;
}

export function mapStructureCollision(
  candidate: Pick<PlannedBuilding, "kind" | "x" | "y">,
  structures: readonly MapStructure[]
): MapStructure | null {
  const candidateRadius = buildingRules[candidate.kind].baseClearance;
  for (const structure of structures) {
    if (structure.collision.shape === "world-square") {
      if (
        circleIntersectsRectangle(
          candidate.x,
          candidate.y,
          candidateRadius,
          structure.x - structure.collision.halfSize,
          structure.y - structure.collision.halfSize,
          structure.x + structure.collision.halfSize,
          structure.y + structure.collision.halfSize
        )
      ) {
        return structure;
      }
      continue;
    }
    const half = ((structure.collision.radiusInCells * 2 + 1) * TERRITORY_UNIT) / 2;
    const centerX = snapTerritoryCoordinate(structure.x);
    const centerY = snapTerritoryCoordinate(structure.y);
    if (
      circleIntersectsRectangle(
        candidate.x,
        candidate.y,
        candidateRadius,
        centerX - half,
        centerY - half,
        centerX + half,
        centerY + half
      )
    ) {
      return structure;
    }
  }
  return null;
}

function pointInTriangle(
  x: number,
  y: number,
  ax: number,
  ay: number,
  bx: number,
  by: number,
  cx: number,
  cy: number
): boolean {
  const first = (x - bx) * (ay - by) - (y - by) * (ax - bx);
  const second = (x - cx) * (by - cy) - (y - cy) * (bx - cx);
  const third = (x - ax) * (cy - ay) - (y - ay) * (cx - ax);
  return (first >= 0 && second >= 0 && third >= 0) || (first <= 0 && second <= 0 && third <= 0);
}

function segmentDistanceSquared(
  px: number,
  py: number,
  ax: number,
  ay: number,
  bx: number,
  by: number
): number {
  const dx = bx - ax;
  const dy = by - ay;
  const lengthSquared = dx * dx + dy * dy;
  if (lengthSquared === 0) return (px - ax) ** 2 + (py - ay) ** 2;
  const amount = Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / lengthSquared));
  const x = ax + amount * dx;
  const y = ay + amount * dy;
  return (px - x) ** 2 + (py - y) ** 2;
}

export function circleIntersectsTriangle(
  x: number,
  y: number,
  radius: number,
  triangle: [number, number, number, number, number, number]
): boolean {
  const [ax, ay, bx, by, cx, cy] = triangle;
  if (pointInTriangle(x, y, ax, ay, bx, by, cx, cy)) return true;
  const radiusSquared = radius * radius;
  return (
    segmentDistanceSquared(x, y, ax, ay, bx, by) <= radiusSquared ||
    segmentDistanceSquared(x, y, bx, by, cx, cy) <= radiusSquared ||
    segmentDistanceSquared(x, y, cx, cy, ax, ay) <= radiusSquared
  );
}

export function transformedTriangle(
  definition: MeshDefinition,
  instance: MeshInstance,
  index: number
): [number, number, number, number, number, number] {
  const [a, b, c, d, tx, ty] = instance.affine;
  const first = definition.vertices[definition.indices[index]];
  const second = definition.vertices[definition.indices[index + 1]];
  const third = definition.vertices[definition.indices[index + 2]];
  return [
    a * first[0] + b * first[1] + tx,
    c * first[0] + d * first[1] + ty,
    a * second[0] + b * second[1] + tx,
    c * second[0] + d * second[1] + ty,
    a * third[0] + b * third[1] + tx,
    c * third[0] + d * third[1] + ty,
  ];
}

export function boundaryCollision(
  x: number,
  y: number,
  radius: number,
  definitions: MeshDefinition[],
  instances: MeshInstance[]
): boolean {
  for (const instance of instances) {
    const definition = definitions[instance.mesh];
    if (!definition) continue;
    for (let index = 0; index < definition.indices.length; index += 3) {
      if (
        circleIntersectsTriangle(x, y, radius, transformedTriangle(definition, instance, index))
      ) {
        return true;
      }
    }
  }
  return false;
}

export function countCoveredResources(
  ownership: TerritoryOwnership,
  resources: ResourcePoint[],
  allianceId: string
): Record<ResourceKind, number> {
  const counts: Record<ResourceKind, number> = { food: 0, wood: 0, stone: 0, coin: 0, crystal: 0 };
  const seen = new Set<number>();
  for (const resource of resources) {
    if (seen.has(resource.id)) continue;
    const owner = ownership.get(
      territoryCellKey(
        Math.floor(resource.x / TERRITORY_UNIT),
        Math.floor(resource.y / TERRITORY_UNIT)
      )
    );
    if (owner?.allianceId === allianceId) {
      seen.add(resource.id);
      counts[resource.kind] += 1;
    }
  }
  return counts;
}

export function countResourcesCoveredByBuilding(
  ownership: TerritoryOwnership,
  building: PlannedBuilding,
  resources: ResourcePoint[]
): Record<ResourceKind, number> {
  const counts: Record<ResourceKind, number> = { food: 0, wood: 0, stone: 0, coin: 0, crystal: 0 };
  const coveredCells = new Set(
    territoryCells(building).map((cell) => territoryCellKey(cell.column, cell.row))
  );
  const seen = new Set<number>();
  for (const resource of resources) {
    if (seen.has(resource.id)) continue;
    const key = territoryCellKey(
      Math.floor(resource.x / TERRITORY_UNIT),
      Math.floor(resource.y / TERRITORY_UNIT)
    );
    if (coveredCells.has(key) && ownership.get(key)?.allianceId === building.allianceId) {
      seen.add(resource.id);
      counts[resource.kind] += 1;
    }
  }
  return counts;
}

export function calculateResourceProduction(
  counts: Record<ResourceKind, number>,
  rates: Record<ResourceKind, number>
): Record<ResourceKind, number> {
  return Object.fromEntries(
    (Object.entries(counts) as Array<[ResourceKind, number]>).map(([kind, count]) => [
      kind,
      count * rates[kind],
    ])
  ) as Record<ResourceKind, number>;
}
