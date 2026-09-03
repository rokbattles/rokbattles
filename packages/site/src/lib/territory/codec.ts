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

const MAGIC = "RTP1";
const HEADER_BYTES = 20;
const FLAG_MASKED = 1;
const KIND_MESH_DEFINITIONS = 1;
const KIND_SPATIAL_CHUNK = 2;
const KIND_PROVINCE_GRID = 3;

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
    if (this.offset >= this.data.length) throw new Error("RTP payload ended unexpectedly");
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
    throw new Error("RTP varint is too long");
  }

  varsint(): number {
    const value = this.varuint();
    return value % 2 === 0 ? value / 2 : -(value + 1) / 2;
  }

  text(): string {
    const length = this.varuint();
    const end = this.offset + length;
    if (end > this.data.length) throw new Error("RTP string extends beyond payload");
    const value = new TextDecoder().decode(this.data.subarray(this.offset, end));
    this.offset = end;
    return value;
  }
}

function unmask(payload: Uint8Array, seed: number): Uint8Array {
  const output = new Uint8Array(payload.length);
  let state = seed >>> 0 || 0x6d2b79f5;
  let word = 0;
  for (let index = 0; index < payload.length; index += 1) {
    if (index % 4 === 0) {
      state ^= state << 13;
      state ^= state >>> 17;
      state ^= state << 5;
      state >>>= 0;
      word = state;
    }
    output[index] = payload[index] ^ ((word >>> ((index % 4) * 8)) & 0xff);
  }
  return output;
}

let crcTable: Uint32Array | null = null;

function crc32(data: Uint8Array): number {
  if (!crcTable) {
    crcTable = new Uint32Array(256);
    for (let index = 0; index < 256; index += 1) {
      let value = index;
      for (let bit = 0; bit < 8; bit += 1) {
        value = (value & 1) !== 0 ? 0xedb88320 ^ (value >>> 1) : value >>> 1;
      }
      crcTable[index] = value >>> 0;
    }
  }
  let checksum = 0xffffffff;
  for (const value of data) checksum = crcTable[(checksum ^ value) & 0xff] ^ (checksum >>> 8);
  return (checksum ^ 0xffffffff) >>> 0;
}

function decodeEnvelope(buffer: ArrayBuffer, expectedKind: number): Uint8Array {
  if (buffer.byteLength < HEADER_BYTES) throw new Error("RTP file is shorter than its header");
  const bytes = new Uint8Array(buffer);
  const magic = new TextDecoder().decode(bytes.subarray(0, 4));
  if (magic !== MAGIC) throw new Error(`Unsupported RTP magic ${magic}`);
  const view = new DataView(buffer);
  const schema = view.getUint8(4);
  const kind = view.getUint8(5);
  const flags = view.getUint8(6);
  const seed = view.getUint32(8, true);
  const length = view.getUint32(12, true);
  const expectedCrc = view.getUint32(16, true);
  if (schema !== 3) throw new Error(`Unsupported RTP schema ${schema}`);
  if (kind !== expectedKind) throw new Error(`Expected RTP kind ${expectedKind}, got ${kind}`);
  const encoded = bytes.subarray(HEADER_BYTES);
  if (encoded.length !== length) throw new Error("RTP payload length does not match header");
  const payload = (flags & FLAG_MASKED) !== 0 ? unmask(encoded, seed) : encoded;
  if (crc32(payload) !== expectedCrc) throw new Error("RTP payload checksum failed");
  return payload;
}

export function decodeMeshDefinitions(buffer: ArrayBuffer): MeshDefinition[] {
  const reader = new Reader(decodeEnvelope(buffer, KIND_MESH_DEFINITIONS));
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

export function decodeSpatialChunk(buffer: ArrayBuffer): SpatialChunk {
  const reader = new Reader(decodeEnvelope(buffer, KIND_SPATIAL_CHUNK));
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
    if (!kind) throw new Error("RTP resource kind is unknown");
    resources.push({ id, kind, x: reader.varsint() / scale, y: reader.varsint() / scale });
  }
  const landmarkCount = reader.varuint();
  const landmarks: MapLandmark[] = [];
  for (let index = 0; index < landmarkCount; index += 1) {
    const id = reader.varuint();
    const kind = landmarkKinds[reader.byte()];
    if (!kind) throw new Error("RTP landmark kind is unknown");
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
    if (!kind) throw new Error("RTP structure kind is unknown");
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
    if (!collision) throw new Error("RTP structure collision shape is unknown");
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
  if (reader.offset !== reader.data.length) throw new Error("RTP spatial chunk has trailing data");
  return { x, y, instances, resources, landmarks, structures };
}

export function decodeProvinceGrid(buffer: ArrayBuffer): ProvinceRestrictionGrid {
  const reader = new Reader(decodeEnvelope(buffer, KIND_PROVINCE_GRID));
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
      throw new Error("RTP province run extends beyond the declared grid");
    }
    cells.fill(provinceId, cellOffset, cellOffset + length);
    cellOffset += length;
  }
  if (cellOffset !== cells.length) throw new Error("RTP province grid is incomplete");
  if (reader.offset !== reader.data.length) throw new Error("RTP province grid has trailing data");
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
