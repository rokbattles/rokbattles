import type { RawOverview } from "@/lib/types/rawReport";

export const OVERVIEW_METRICS = [
  { label: "Troop Units", selfKey: "max", enemyKey: "enemy_max" },
  { label: "Dead", selfKey: "death", enemyKey: "enemy_death" },
  {
    label: "Severely Wounded",
    selfKey: "severely_wounded",
    enemyKey: "enemy_severely_wounded",
  },
  { label: "Slightly Wounded", selfKey: "wounded", enemyKey: "enemy_wounded" },
  { label: "Remaining", selfKey: "remaining", enemyKey: "enemy_remaining" },
  { label: "Kill Points", selfKey: "kill_score", enemyKey: "enemy_kill_score" },
] as const satisfies readonly {
  label: string;
  selfKey: keyof RawOverview;
  enemyKey: keyof RawOverview;
}[];

export function getOverviewValue(overview: RawOverview, key: keyof RawOverview) {
  const value = overview?.[key];
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return null;
  }
  return value;
}

export function hasOverviewData(overview: RawOverview) {
  return OVERVIEW_METRICS.some((metric) => {
    const selfValue = getOverviewValue(overview, metric.selfKey);
    const enemyValue = getOverviewValue(overview, metric.enemyKey);
    return selfValue != null || enemyValue != null;
  });
}
