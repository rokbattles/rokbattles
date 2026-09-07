export const LOST_KINGDOM_TERRITORY_COLORS = [
  { id: 1, name: "Red", value: "#FF0000" },
  { id: 2, name: "Orange", value: "#FF8A00" },
  { id: 3, name: "Yellow", value: "#FFFC00" },
  { id: 4, name: "Green", value: "#00FF5A" },
  { id: 5, name: "Cyan", value: "#00DEFF" },
  { id: 6, name: "Blue", value: "#0084FF" },
  { id: 7, name: "Purple", value: "#B400FF" },
  { id: 8, name: "Pink", value: "#FF4DBE" },
] as const;

const REAL_UNITS_PER_GAME_COORDINATE = 6;
const SEASON_PREFIX = /^Season \d+:\s*/;
const PREPARATION_SEASON = /^Preparation Season ([1-4])$/;
const LOST_KINGDOM_COLOR_BY_VALUE = new Map(
  LOST_KINGDOM_TERRITORY_COLORS.map(({ value }) => [value.toLowerCase(), value])
);

export function mapDisplayTitle(title: string): string {
  const home = title.match(PREPARATION_SEASON);
  return home ? `Home ${home[1]}` : title.replace(SEASON_PREFIX, "");
}

export function normalizeLostKingdomTerritoryColor(value: unknown, fallbackIndex = 0): string {
  const matchingColor =
    typeof value === "string" ? LOST_KINGDOM_COLOR_BY_VALUE.get(value.toLowerCase()) : undefined;
  if (matchingColor) return matchingColor;

  const colorCount = LOST_KINGDOM_TERRITORY_COLORS.length;
  const normalizedIndex = ((Math.trunc(fallbackIndex) % colorCount) + colorCount) % colorCount;
  return (LOST_KINGDOM_TERRITORY_COLORS[normalizedIndex] ?? LOST_KINGDOM_TERRITORY_COLORS[0]).value;
}

export function realToGameCoordinate(value: number): number {
  return Math.floor(value / REAL_UNITS_PER_GAME_COORDINATE + 0.5);
}

export function realToGamePoint(point: { x: number; y: number }): { x: number; y: number } {
  return {
    x: realToGameCoordinate(point.x),
    y: realToGameCoordinate(point.y),
  };
}
