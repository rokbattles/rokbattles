import { normalizeLostKingdomTerritoryColor } from "./presentation";
import type {
  Alliance,
  BuildingKind,
  PlannedBuilding,
  PlannedDrawing,
  PlannerDocument,
} from "./types";

const BUILDING_KINDS = new Set<BuildingKind>(["flag", "mainFortress", "subFortress", "horse"]);

function record(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === "object" ? (value as Record<string, unknown>) : null;
}

function finiteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function normalizeAlliances(value: unknown): Alliance[] | null {
  if (!Array.isArray(value) || value.length === 0) return null;
  const alliances: Alliance[] = [];
  for (const [index, candidate] of value.entries()) {
    const alliance = record(candidate);
    if (
      !alliance ||
      typeof alliance.id !== "string" ||
      typeof alliance.name !== "string" ||
      typeof alliance.color !== "string"
    ) {
      return null;
    }
    alliances.push({
      id: alliance.id,
      name: alliance.name,
      color: normalizeLostKingdomTerritoryColor(alliance.color, index),
    });
  }
  return alliances;
}

function normalizeBuildings(
  value: unknown,
  allianceIds: ReadonlySet<string>
): PlannedBuilding[] | null {
  if (!Array.isArray(value)) return null;
  const buildings: PlannedBuilding[] = [];
  for (const candidate of value) {
    const building = record(candidate);
    if (
      !building ||
      typeof building.id !== "string" ||
      typeof building.allianceId !== "string" ||
      !allianceIds.has(building.allianceId) ||
      typeof building.kind !== "string" ||
      !BUILDING_KINDS.has(building.kind as BuildingKind) ||
      !finiteNumber(building.x) ||
      !finiteNumber(building.y)
    ) {
      return null;
    }
    buildings.push({
      id: building.id,
      allianceId: building.allianceId,
      kind: building.kind as BuildingKind,
      x: building.x,
      y: building.y,
    });
  }
  return buildings;
}

function normalizeDrawings(
  value: unknown,
  allianceIds: ReadonlySet<string>
): PlannedDrawing[] | null {
  if (!Array.isArray(value)) return null;
  const drawings: PlannedDrawing[] = [];
  for (const candidate of value) {
    const drawing = record(candidate);
    if (
      !drawing ||
      typeof drawing.id !== "string" ||
      typeof drawing.allianceId !== "string" ||
      !allianceIds.has(drawing.allianceId) ||
      !Array.isArray(drawing.points) ||
      drawing.points.length < 2 ||
      drawing.points.length > 10_000
    ) {
      return null;
    }
    const points: PlannedDrawing["points"] = [];
    for (const candidatePoint of drawing.points) {
      const point = record(candidatePoint);
      if (!point || !finiteNumber(point.x) || !finiteNumber(point.y)) return null;
      points.push({ x: point.x, y: point.y });
    }
    drawings.push({ id: drawing.id, allianceId: drawing.allianceId, points });
  }
  return drawings;
}

export function normalizePlannerDocument(
  value: unknown,
  availableMapSlugs: ReadonlySet<string>
): PlannerDocument | null {
  const document = record(value);
  if (
    !document ||
    (document.version !== 1 && document.version !== 2) ||
    typeof document.mapSlug !== "string" ||
    !availableMapSlugs.has(document.mapSlug) ||
    typeof document.activeAllianceId !== "string"
  ) {
    return null;
  }
  const alliances = normalizeAlliances(document.alliances);
  if (!alliances) return null;
  const allianceIds = new Set(alliances.map((alliance) => alliance.id));
  const buildings = normalizeBuildings(document.buildings, allianceIds);
  if (!buildings) return null;
  const drawings = document.version === 1 ? [] : normalizeDrawings(document.drawings, allianceIds);
  if (!drawings) return null;
  return {
    version: 2,
    mapSlug: document.mapSlug,
    activeAllianceId: allianceIds.has(document.activeAllianceId)
      ? document.activeAllianceId
      : alliances[0].id,
    alliances,
    buildings,
    drawings,
  };
}

export function encodePlan(document: PlannerDocument): string {
  const bytes = new TextEncoder().encode(JSON.stringify(document));
  let binary = "";
  for (let index = 0; index < bytes.length; index += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(index, index + 0x8000));
  }
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

export function decodePlan(value: string): unknown {
  const padded = value
    .replaceAll("-", "+")
    .replaceAll("_", "/")
    .padEnd(Math.ceil(value.length / 4) * 4, "=");
  const binary = atob(padded);
  const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
  return JSON.parse(new TextDecoder().decode(bytes));
}
