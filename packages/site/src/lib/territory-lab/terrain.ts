import type { MapData } from "./types";

type Terrain = Pick<MapData, "definitions" | "instances">;
type Bounds = { minX: number; minY: number; maxX: number; maxY: number };

// A loaded dataset is immutable for the editor's lifetime; disposal releases this entry.
const cache = new WeakMap<Terrain, Bounds[]>();

/** Conservative world bounds keep placement checks out of distant triangle meshes. */
export function nearbyTerrainInstances(data: Terrain, x: number, y: number, radius: number) {
  let bounds = cache.get(data);

  if (!bounds) {
    const definitions = data.definitions.map((definition) => {
      const box = { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity };

      for (const [vx, vy] of definition.vertices) {
        box.minX = Math.min(box.minX, vx);
        box.minY = Math.min(box.minY, vy);
        box.maxX = Math.max(box.maxX, vx);
        box.maxY = Math.max(box.maxY, vy);
      }

      return box;
    });

    bounds = data.instances.map((instance) => {
      const source = definitions[instance.mesh];
      const box = { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity };
      if (!source) return box;

      const [a, b, c, d, tx, ty] = instance.affine;

      for (const vx of [source.minX, source.maxX]) {
        for (const vy of [source.minY, source.maxY]) {
          const wx = a * vx + b * vy + tx;
          const wy = c * vx + d * vy + ty;

          box.minX = Math.min(box.minX, wx);
          box.minY = Math.min(box.minY, wy);
          box.maxX = Math.max(box.maxX, wx);
          box.maxY = Math.max(box.maxY, wy);
        }
      }

      return box;
    });

    cache.set(data, bounds);
  }

  return data.instances.filter((_, index) => {
    const box = bounds[index];

    return (
      x + radius >= box.minX &&
      y + radius >= box.minY &&
      x - radius <= box.maxX &&
      y - radius <= box.maxY
    );
  });
}
