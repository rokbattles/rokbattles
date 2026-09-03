import { getGameMapUrl, getTerritoryDataBaseUrl } from "../game-sprite";
import { decodeMeshDefinitions, decodeProvinceGrid, decodeSpatialChunk } from "./codec";
import { buildingRules } from "./geometry";
import type {
  BuildingCostSchedule,
  BuildingKind,
  MapStructure,
  MeshDefinition,
  PlannedBuilding,
  ProvinceRestrictionGrid,
  ResourcePoint,
  SpatialChunk,
  TerritoryApiBuildingConfig,
  TerritoryApiBuildingKind,
  TerritoryMapApiConfig,
} from "./types";

export type WorldBounds = { minX: number; minY: number; maxX: number; maxY: number };

function key(x: number, y: number): string {
  return `${x}:${y}`;
}

async function fetchJson<T>(url: string, signal?: AbortSignal): Promise<T> {
  const response = await fetch(url, { signal });
  if (!response.ok) throw new Error(`Could not load ${url} (${response.status})`);
  return (await response.json()) as T;
}

async function fetchBuffer(url: string, signal?: AbortSignal): Promise<ArrayBuffer> {
  const response = await fetch(url, { signal });
  if (!response.ok) throw new Error(`Could not load ${url} (${response.status})`);
  return response.arrayBuffer();
}

function loadImage(url: string, signal?: AbortSignal): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    let settled = false;

    const cleanup = () => {
      image.onload = null;
      image.onerror = null;
      signal?.removeEventListener("abort", abort);
    };
    const finish = (result: () => void) => {
      if (settled) return;
      settled = true;
      cleanup();
      result();
    };
    const abort = () => {
      finish(() => {
        image.src = "data:,";
        reject(new DOMException("The map image request was aborted.", "AbortError"));
      });
    };

    if (signal?.aborted) {
      abort();
      return;
    }

    image.crossOrigin = "anonymous";
    image.decoding = "async";
    image.onload = () => finish(() => resolve(image));
    image.onerror = () => finish(() => reject(new Error(`Could not load the map image ${url}`)));
    signal?.addEventListener("abort", abort, { once: true });
    image.src = url;
  });
}

export class TerritoryDataSource {
  readonly config: TerritoryMapApiConfig;
  readonly costs: BuildingCostSchedule = {};
  readonly buildings: Partial<Record<BuildingKind, TerritoryApiBuildingConfig>> = {};
  readonly productionRates: Record<ResourcePoint["kind"], number>;
  readonly definitions: MeshDefinition[];
  readonly provinceGrid: ProvinceRestrictionGrid | null;
  readonly baseUrl: string;
  readonly image: HTMLImageElement;
  readonly chunks = new Map<string, SpatialChunk>();
  readonly chunkPromises = new Map<string, Promise<SpatialChunk | null>>();
  readonly chunkPaths = new Map<string, string>();

  private constructor(
    config: TerritoryMapApiConfig,
    definitions: MeshDefinition[],
    provinceGrid: ProvinceRestrictionGrid | null,
    assetBaseUrl: string,
    image: HTMLImageElement
  ) {
    this.config = config;
    this.definitions = definitions;
    this.provinceGrid = provinceGrid;
    this.baseUrl = assetBaseUrl;
    this.image = image;
    this.productionRates = {
      food: config.resourceProductionPerHour.food,
      wood: config.resourceProductionPerHour.wood,
      stone: config.resourceProductionPerHour.stone,
      coin: config.resourceProductionPerHour.gold,
      crystal: config.resourceProductionPerHour.crystal,
    };
    for (const [apiKind, building] of Object.entries(config.buildings) as Array<
      [TerritoryApiBuildingKind, TerritoryApiBuildingConfig]
    >) {
      this.buildings[API_BUILDING_KIND[apiKind]] = building;
    }
    for (const [apiKind, tiers] of Object.entries(config.costs) as Array<
      [TerritoryApiBuildingKind, NonNullable<BuildingCostSchedule[BuildingKind]>]
    >) {
      this.costs[API_BUILDING_KIND[apiKind]] = tiers;
    }
    for (const [x, y] of config.spatial.chunks) {
      this.chunkPaths.set(key(x, y), `chunks/${x}_${y}.rtp`);
    }
  }

  static async load(mapSlug: string, signal?: AbortSignal): Promise<TerritoryDataSource> {
    const config = await fetchJson<TerritoryMapApiConfig>(
      `/proxy/v1/global/territory-planner/map/${encodeURIComponent(mapSlug)}`,
      signal
    );
    if (config.slug !== mapSlug) throw new Error("Territory Planner API returned the wrong map");
    if (config.schemaVersion !== 1) {
      throw new Error(`Unsupported Territory Planner config version ${config.schemaVersion}`);
    }
    const baseUrl = getTerritoryDataBaseUrl(config.slug);
    const imageUrl = getGameMapUrl(config.imageFile);
    const [definitionsBuffer, provinceBuffer, image] = await Promise.all([
      fetchBuffer(`${baseUrl}mesh.rtp`, signal),
      config.spatial.province
        ? fetchBuffer(`${baseUrl}province.rtp`, signal)
        : Promise.resolve(null),
      loadImage(imageUrl, signal),
    ]);
    return new TerritoryDataSource(
      config,
      decodeMeshDefinitions(definitionsBuffer),
      provinceBuffer ? decodeProvinceGrid(provinceBuffer) : null,
      baseUrl,
      image
    );
  }

  async loadChunk(x: number, y: number, signal?: AbortSignal): Promise<SpatialChunk | null> {
    const chunkKey = key(x, y);
    const cached = this.chunks.get(chunkKey);
    if (cached) return cached;
    const path = this.chunkPaths.get(chunkKey);
    if (!path) return null;
    const existing = this.chunkPromises.get(chunkKey);
    if (existing) return existing;
    const promise = fetchBuffer(`${this.baseUrl}${path}`, signal)
      .then((buffer) => {
        const chunk = decodeSpatialChunk(buffer);
        this.chunks.set(chunkKey, chunk);
        return chunk;
      })
      .finally(() => this.chunkPromises.delete(chunkKey));
    this.chunkPromises.set(chunkKey, promise);
    return promise;
  }

  chunkCoordinates(
    bounds: WorldBounds,
    buffer = this.config.spatial.chunkBuffer
  ): Array<[number, number]> {
    const size = this.config.spatial.chunkSize;
    const minX = Math.floor(bounds.minX / size) - buffer;
    const maxX = Math.floor(bounds.maxX / size) + buffer;
    const minY = Math.floor(bounds.minY / size) - buffer;
    const maxY = Math.floor(bounds.maxY / size) + buffer;
    const coordinates: Array<[number, number]> = [];
    for (let y = minY; y <= maxY; y += 1) {
      for (let x = minX; x <= maxX; x += 1) {
        if (this.chunkPaths.has(key(x, y))) coordinates.push([x, y]);
      }
    }
    return coordinates;
  }

  async ensureBounds(bounds: WorldBounds, signal?: AbortSignal, buffer?: number): Promise<void> {
    await Promise.all(
      this.chunkCoordinates(bounds, buffer).map(([x, y]) => this.loadChunk(x, y, signal))
    );
  }

  async ensureBuildings(buildings: PlannedBuilding[], signal?: AbortSignal): Promise<void> {
    const coordinates = new Set<string>();
    for (const building of buildings) {
      const half = buildingRules[building.kind].territorySide / 2;
      for (const [x, y] of this.chunkCoordinates(
        {
          minX: building.x - half,
          minY: building.y - half,
          maxX: building.x + half,
          maxY: building.y + half,
        },
        1
      )) {
        coordinates.add(key(x, y));
      }
    }
    await Promise.all(
      [...coordinates].map((chunkKey) => {
        const [x, y] = chunkKey.split(":").map(Number);
        return this.loadChunk(x, y, signal);
      })
    );
  }

  chunksWithin(bounds: WorldBounds, buffer = this.config.spatial.chunkBuffer): SpatialChunk[] {
    return this.chunkCoordinates(bounds, buffer)
      .map(([x, y]) => this.chunks.get(key(x, y)))
      .filter((chunk): chunk is SpatialChunk => Boolean(chunk));
  }

  allLoadedResources(): ResourcePoint[] {
    const resources: ResourcePoint[] = [];
    for (const chunk of this.chunks.values()) resources.push(...chunk.resources);
    return resources;
  }

  allLoadedStructures(): MapStructure[] {
    const structures: MapStructure[] = [];
    for (const chunk of this.chunks.values()) structures.push(...chunk.structures);
    return structures;
  }
}

const API_BUILDING_KIND: Record<TerritoryApiBuildingKind, BuildingKind> = {
  flag: "flag",
  centerFortress: "mainFortress",
  allianceFortress: "subFortress",
  horse: "horse",
};
