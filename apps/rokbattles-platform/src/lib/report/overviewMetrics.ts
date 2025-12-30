import type { RawOverview } from "@/lib/types/rawReport";

export const OVERVIEW_METRICS = [
  { labelKey: "troopUnits", selfKey: "max", enemyKey: "enemy_max" },
  { labelKey: "dead", selfKey: "death", enemyKey: "enemy_death" },
  {
    labelKey: "severelyWounded",
    selfKey: "severely_wounded",
    enemyKey: "enemy_severely_wounded",
  },
  { labelKey: "slightlyWounded", selfKey: "wounded", enemyKey: "enemy_wounded" },
  { labelKey: "remaining", selfKey: "remaining", enemyKey: "enemy_remaining" },
  { labelKey: "killPoints", selfKey: "kill_score", enemyKey: "enemy_kill_score" },
] as const satisfies readonly {
  labelKey: string;
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
