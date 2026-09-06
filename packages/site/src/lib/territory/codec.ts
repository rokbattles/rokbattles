import { readContainerPayload } from "../container";
import type {
  LandmarkKind,
  MapLandmark,
  MapStructure,
  MapStructureKind,
  MeshDefinition,
  MeshInstance,
  ProvinceRestrictionGrid,
  ResourceKind,
  ResourcePoint,
  SpatialChunk,
} from "./types";

// These IDs identify the quantized, varint-encoded planner payload layouts.
const SCHEMA_MESH_DEFINITIONS = 401;
const SCHEMA_SPATIAL_CHUNK = 402;
const SCHEMA_PROVINCE_GRID = 403;

const resourceKinds: ResourceKind[] = ["food", "food", "wood", "stone", "coin", "crystal"];
const landmarkKinds: LandmarkKind[] = ["village", "village", "cave"];
const structureKinds: MapStructureKind[] = [
  "pass",
  "pass",
  "holy-site",
  "ancient-battlefield",
  "bastion",
];

class Reader {
  readonly data: Uint8Array;
  offset = 0;

  constructor(data: Uint8Array) {
    this.data = data;
  }

  byte(): number {
    if (this.offset >= this.data.length) throw new Error("Territory payload ended unexpectedly");
    return this.data[this.offset++];
  }

  varuint(): number {
    let result = 0;
    let multiplier = 1;
    for (let count = 0; count < 10; count += 1) {
      const value = this.byte();
      result += (value & 0x7f) * multiplier;
      if ((value & 0x80) === 0) return result;
      multiplier *= 128;
    }
    throw new Error("Territory varint is too long");
  }

  varsint(): number {
    const value = this.varuint();
    return value % 2 === 0 ? value / 2 : -(value + 1) / 2;
  }

  text(): string {
    const length = this.varuint();
    const end = this.offset + length;
    if (end > this.data.length) throw new Error("Territory string extends beyond payload");
    const value = new TextDecoder().decode(this.data.subarray(this.offset, end));
    this.offset = end;
    return value;
  }
}

export async function decodeMeshDefinitions(buffer: ArrayBuffer): Promise<MeshDefinition[]> {
  const reader = new Reader(await readContainerPayload(buffer, SCHEMA_MESH_DEFINITIONS));
  const scale = reader.varuint();
  const count = reader.varuint();
  const definitions: MeshDefinition[] = [];
  for (let definitionIndex = 0; definitionIndex < count; definitionIndex += 1) {
    const id = reader.varuint();
    const name = reader.text();
    const vertexCount = reader.varuint();
    const vertices: Array<[number, number]> = [];
    let x = 0;
    let y = 0;
    for (let vertexIndex = 0; vertexIndex < vertexCount; vertexIndex += 1) {
      x += reader.varsint();
      y += reader.varsint();
      vertices.push([x / scale, y / scale]);
    }
    const indexCount = reader.varuint();
    const indices = Array.from({ length: indexCount }, () => reader.varuint());
    definitions.push({ id, name, vertices, indices });
  }
  return definitions;
}

export async function decodeSpatialChunk(buffer: ArrayBuffer): Promise<SpatialChunk> {
  const reader = new Reader(await readContainerPayload(buffer, SCHEMA_SPATIAL_CHUNK));
  const scale = reader.varuint();
  const x = reader.varsint();
  const y = reader.varsint();
  const instanceCount = reader.varuint();
  const instances: MeshInstance[] = [];
  for (let index = 0; index < instanceCount; index += 1) {
    const mesh = reader.varuint();
    const affine = Array.from(
      { length: 6 },
      () => reader.varsint() / scale
    ) as MeshInstance["affine"];
    instances.push({ mesh, affine });
  }
  const resourceCount = reader.varuint();
  const resources: ResourcePoint[] = [];
  for (let index = 0; index < resourceCount; index += 1) {
    const id = reader.varuint();
    const kind = resourceKinds[reader.byte()];
    if (!kind) throw new Error("Territory resource kind is unknown");
    resources.push({ id, kind, x: reader.varsint() / scale, y: reader.varsint() / scale });
  }
  const landmarkCount = reader.varuint();
  const landmarks: MapLandmark[] = [];
  for (let index = 0; index < landmarkCount; index += 1) {
    const id = reader.varuint();
    const kind = landmarkKinds[reader.byte()];
    if (!kind) throw new Error("Territory landmark kind is unknown");
    landmarks.push({
      id,
      kind,
      x: reader.varsint() / scale,
      y: reader.varsint() / scale,
    });
  }
  const structureCount = reader.varuint();
  const structures: MapStructure[] = [];
  for (let index = 0; index < structureCount; index += 1) {
    const id = reader.varuint();
    const strongholdType = reader.varuint();
    const kind = structureKinds[reader.byte()];
    if (!kind) throw new Error("Territory structure kind is unknown");
    const structureX = reader.varsint() / scale;
    const structureY = reader.varsint() / scale;
    const collisionShape = reader.byte();
    const collisionAmount = reader.varuint();
    const territoryRadiusInCells = reader.varuint();
    const encodedTeleportRadius = reader.varuint();
    const flags = reader.byte();
    const label = reader.text();
    const collision =
      collisionShape === 1
        ? ({ shape: "world-square", halfSize: collisionAmount / scale } as const)
        : collisionShape === 2
          ? ({ shape: "territory-square", radiusInCells: collisionAmount } as const)
          : null;
    if (!collision) throw new Error("Territory structure collision shape is unknown");
    structures.push({
      id,
      strongholdType,
      kind,
      x: structureX,
      y: structureY,
      label,
      collision,
      territoryRadiusInCells,
      teleportRadiusInCells: encodedTeleportRadius === 0 ? null : encodedTeleportRadius - 1,
      claimable: (flags & 1) !== 0,
    });
  }
  if (reader.offset !== reader.data.length)
    throw new Error("Territory spatial chunk has trailing data");
  return { x, y, instances, resources, landmarks, structures };
}

export async function decodeProvinceGrid(buffer: ArrayBuffer): Promise<ProvinceRestrictionGrid> {
  const reader = new Reader(await readContainerPayload(buffer, SCHEMA_PROVINCE_GRID));
  const serverSchema = reader.varuint();
  const effectiveSchema = reader.varuint();
  const cellSize = reader.varuint();
  const width = reader.varuint();
  const height = reader.varuint();
  const readBlocked = (): ReadonlySet<number> => {
    const count = reader.varuint();
    return new Set(Array.from({ length: count }, () => reader.varuint()));
  };
  const flagBlocked = readBlocked();
  const fortressBlocked = readBlocked();
  const cells = new Uint8Array(width * height);
  let cellOffset = 0;
  const runCount = reader.varuint();
  for (let runIndex = 0; runIndex < runCount; runIndex += 1) {
    const length = reader.varuint();
    const provinceId = reader.byte();
    if (length === 0 || cellOffset + length > cells.length) {
      throw new Error("Territory province run extends beyond the declared grid");
    }
    cells.fill(provinceId, cellOffset, cellOffset + length);
    cellOffset += length;
  }
  if (cellOffset !== cells.length) throw new Error("Territory province grid is incomplete");
  if (reader.offset !== reader.data.length)
    throw new Error("Territory province grid has trailing data");
  return {
    cellSize,
    width,
    height,
    serverSchema,
    effectiveSchema,
    cells,
    flagBlocked,
    fortressBlocked,
  };
}
