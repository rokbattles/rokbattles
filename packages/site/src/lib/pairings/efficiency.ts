import type { PairingAggregate } from "@/lib/pairings";
import { casualties } from "@/lib/pairings/analytics";

// Bounded axes keep breakeven and typical cost centered without an outlier
// compressing every other march. Icon labels have separate, non-overlapping slots.
export function efficiencyPoints(items: PairingAggregate[], width: number, baseHeight: number) {
  const costs = items.map((item) => casualties(item.totals) / item.count).toSorted((a, b) => a - b);
  const median = costs.length
    ? (costs[Math.floor((costs.length - 1) / 2)] + costs[Math.floor(costs.length / 2)]) / 2
    : 0;
  const pivot = median > 0 ? median : 1;

  const positions = items.map((item) => {
    const cost = casualties(item.totals) / item.count;
    const x = cost / (cost + pivot);

    const total = item.totals.killScore + item.totals.enemyKillScore;
    const y = total > 0 ? item.totals.enemyKillScore / total : 0.5;
    return { item, x, y };
  });

  return { ...layoutPairingMarkers(positions, width, baseHeight), median, pivot };
}

type PairingPlotAnchor = { item: PairingAggregate; x: number; y: number };
export type PairingPlotPoint = {
  item: PairingAggregate;
  anchorX: number;
  anchorY: number;
  x: number;
  y: number;
};

function pairingMarkerSize(width: number) {
  return width < 500 ? { width: 52, height: 32 } : { width: 72, height: 44 };
}

type Segment = { x1: number; y1: number; x2: number; y2: number };
type Box = { left: number; right: number; top: number; bottom: number };

export function pairingConnector(point: PairingPlotPoint): Segment {
  return {
    x1: point.x,
    y1: point.y,
    x2: point.anchorX,
    y2: point.anchorY,
  };
}

function intersectsBox(line: Segment, box: Box) {
  let start = 0,
    end = 1;

  for (const [origin, delta, min, max] of [
    [line.x1, line.x2 - line.x1, box.left - 2, box.right + 2],
    [line.y1, line.y2 - line.y1, box.top - 2, box.bottom + 2],
  ]) {
    if (delta === 0) {
      if (origin < min || origin > max) return false;
    } else {
      const a = (min - origin) / delta,
        b = (max - origin) / delta;
      start = Math.max(start, Math.min(a, b));
      end = Math.min(end, Math.max(a, b));
      if (start > end) return false;
    }
  }

  return true;
}

// Reserve the actual data points as well as the icon rectangles. Dense groups can
// move their labels, while the plotted positions remain visible and unchanged.
function layoutPairingMarkers(anchors: PairingPlotAnchor[], width: number, baseHeight: number) {
  const size = pairingMarkerSize(width);
  const quadrants = width >= 2 * (size.width + 16);
  const columns = Math.max(1, Math.floor((width / (quadrants ? 2 : 1) - 16) / (size.width + 8)));
  const quadrant = (point: PairingPlotAnchor) =>
    quadrants ? (point.x > 0.5 ? 1 : 0) + (point.y > 0.5 ? 2 : 0) : 0;
  const counts = [0, 1, 2, 3].map(
    (key) => anchors.filter((point) => quadrant(point) === key).length
  );

  let height = Math.max(
    baseHeight,
    (quadrants ? 2 : 1) * ((Math.ceil(Math.max(...counts) / columns) + 1) * (size.height + 8) + 16)
  );
  let points: PairingPlotPoint[] | null = null;

  while (!points) {
    points = placeMarkers(anchors, width, height, quadrant, quadrants);
    if (!points) height += (size.height + 8) * 2;
  }

  return { points, height };
}

function placeMarkers(
  anchors: PairingPlotAnchor[],
  width: number,
  height: number,
  quadrant: (point: PairingPlotAnchor) => number,
  quadrants: boolean
): PairingPlotPoint[] | null {
  const size = pairingMarkerSize(width);
  const halfWidth = size.width / 2,
    halfHeight = size.height / 2;

  const positions = anchors.map((point, index) => ({
    ...point,
    index,
    anchorX: point.x * width,
    anchorY: point.y * height,
  }));

  const placed: Array<PairingPlotPoint & { index: number }> = [];
  const boxes: Box[] = [],
    lines: Segment[] = [];

  for (const point of positions.toSorted((a, b) => a.y - b.y || a.x - b.x)) {
    const key = quadrant(point);
    const bounds = quadrants
      ? {
          left: ((key % 2) * width) / 2 + 8,
          right: (((key % 2) + 1) * width) / 2 - 8,
          top: (Math.floor(key / 2) * height) / 2 + 8,
          bottom: ((Math.floor(key / 2) + 1) * height) / 2 - 8,
        }
      : { left: 8, right: width - 8, top: 8, bottom: height - 8 };

    const clampX = (x: number) =>
      Math.max(bounds.left + halfWidth, Math.min(bounds.right - halfWidth, x));
    const clampY = (y: number) =>
      Math.max(bounds.top + halfHeight, Math.min(bounds.bottom - halfHeight, y));
    const xs = new Set([clampX(point.anchorX), bounds.left + halfWidth, bounds.right - halfWidth]);
    const ys = new Set([
      clampY(point.anchorY),
      bounds.top + halfHeight,
      bounds.bottom - halfHeight,
    ]);

    for (const anchor of positions) {
      xs.add(clampX(anchor.anchorX - halfWidth - 8));
      xs.add(clampX(anchor.anchorX + halfWidth + 8));
      ys.add(clampY(anchor.anchorY - halfHeight - 8));
      ys.add(clampY(anchor.anchorY + halfHeight + 8));
    }

    for (const box of boxes) {
      xs.add(clampX(box.left - halfWidth - 6));
      xs.add(clampX(box.right + halfWidth + 6));
      ys.add(clampY(box.top - halfHeight - 6));
      ys.add(clampY(box.bottom + halfHeight + 6));
    }
    for (let y = bounds.top + halfHeight; y <= bounds.bottom - halfHeight; y += size.height + 8)
      ys.add(y);

    let best: {
      point: PairingPlotPoint & { index: number };
      box: Box;
      line: Segment;
      score: number;
    } | null = null;

    for (const x of xs)
      for (const y of ys) {
        const box = {
          left: x - halfWidth,
          right: x + halfWidth,
          top: y - halfHeight,
          bottom: y + halfHeight,
        };
        if (
          positions.some(
            (anchor) =>
              anchor.anchorX > box.left - 7 &&
              anchor.anchorX < box.right + 7 &&
              anchor.anchorY > box.top - 7 &&
              anchor.anchorY < box.bottom + 7
          )
        )
          continue;
        if (
          boxes.some(
            (other) =>
              box.left < other.right + 6 &&
              box.right > other.left - 6 &&
              box.top < other.bottom + 6 &&
              box.bottom > other.top - 6
          )
        )
          continue;

        const candidate = {
          item: point.item,
          index: point.index,
          anchorX: point.anchorX,
          anchorY: point.anchorY,
          x,
          y,
        };
        const line = pairingConnector(candidate);
        const crossings =
          boxes.filter((other) => intersectsBox(line, other)).length +
          lines.filter((other) => intersectsBox(other, box)).length;
        const score = Math.hypot(x - point.anchorX, y - point.anchorY) + crossings * size.height;
        if (!best || score < best.score) best = { point: candidate, box, line, score };
      }

    if (!best) return null;
    placed.push(best.point);
    boxes.push(best.box);
    lines.push(best.line);
  }

  return placed.toSorted((a, b) => a.index - b.index);
}
