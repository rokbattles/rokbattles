const REAL_UNITS_PER_GAME_COORDINATE = 6;

export function realToGameCoordinate(value: number): number {
  return Math.floor(value / REAL_UNITS_PER_GAME_COORDINATE + 0.5);
}

export function realToGamePoint(point: { x: number; y: number }): { x: number; y: number } {
  return {
    x: realToGameCoordinate(point.x),
    y: realToGameCoordinate(point.y),
  };
}
