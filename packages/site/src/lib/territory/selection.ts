import type { DrawingPoint, PlannedDrawing, PlannerDocument } from "./types";

function pointToSegmentDistanceSquared(
  point: DrawingPoint,
  start: DrawingPoint,
  end: DrawingPoint
): number {
  const segmentX = end.x - start.x;
  const segmentY = end.y - start.y;
  if (segmentX === 0 && segmentY === 0) {
    return (point.x - start.x) ** 2 + (point.y - start.y) ** 2;
  }

  const progress = Math.max(
    0,
    Math.min(
      1,
      ((point.x - start.x) * segmentX + (point.y - start.y) * segmentY) /
        (segmentX ** 2 + segmentY ** 2)
    )
  );
  const nearestX = start.x + progress * segmentX;
  const nearestY = start.y + progress * segmentY;
  return (point.x - nearestX) ** 2 + (point.y - nearestY) ** 2;
}

export function findDrawingAtPoint(
  drawings: readonly PlannedDrawing[],
  point: DrawingPoint,
  tolerance: number
): string | null {
  let closestId: string | null = null;
  let closestDistanceSquared = Math.max(0, tolerance) ** 2;

  for (let drawingIndex = drawings.length - 1; drawingIndex >= 0; drawingIndex -= 1) {
    const drawing = drawings[drawingIndex];
    for (let pointIndex = 1; pointIndex < drawing.points.length; pointIndex += 1) {
      const distanceSquared = pointToSegmentDistanceSquared(
        point,
        drawing.points[pointIndex - 1],
        drawing.points[pointIndex]
      );
      if (
        distanceSquared < closestDistanceSquared ||
        (closestId === null && distanceSquared === closestDistanceSquared)
      ) {
        closestDistanceSquared = distanceSquared;
        closestId = drawing.id;
      }
    }
  }

  return closestId;
}

export function updatePlannerSelection(
  current: ReadonlySet<string>,
  itemId: string | null,
  additive: boolean
): Set<string> {
  if (!itemId) return additive ? new Set(current) : new Set();
  if (!additive) return new Set([itemId]);

  const next = new Set(current);
  if (next.has(itemId)) next.delete(itemId);
  else next.add(itemId);
  return next;
}

export function deleteSelectedPlannerItems(
  document: PlannerDocument,
  selectedItemIds: ReadonlySet<string>
): PlannerDocument {
  if (selectedItemIds.size === 0) return document;
  const buildings = document.buildings.filter((building) => !selectedItemIds.has(building.id));
  const drawings = document.drawings.filter((drawing) => !selectedItemIds.has(drawing.id));
  if (
    buildings.length === document.buildings.length &&
    drawings.length === document.drawings.length
  ) {
    return document;
  }
  return { ...document, buildings, drawings };
}
