export const combatLabPreviewRangeKeys = ["1y", "6m", "1m", "7d"] as const;
export type CombatLabPreviewRangeKey = (typeof combatLabPreviewRangeKeys)[number];

export const combatLabPreviewScenarioKeys = [
  "all",
  "openField",
  "swarming",
  "rally",
  "garrison",
] as const;
export type CombatLabPreviewScenarioKey = (typeof combatLabPreviewScenarioKeys)[number];

export type CombatLabPreviewCategoryScore = {
  value: number;
  p10: number;
  p90: number;
  score: number;
};

export type CombatLabPreviewDrastc = {
  samples: number;
  breakdown: {
    damage: CombatLabPreviewCategoryScore;
    rage: CombatLabPreviewCategoryScore;
    assist: CombatLabPreviewCategoryScore;
    sustainability: CombatLabPreviewCategoryScore;
    trade: CombatLabPreviewCategoryScore;
    consistency: CombatLabPreviewCategoryScore;
  };
  overall: number;
  confidence: {
    score: number;
    unique_governors: number;
    effective_governors: number;
  };
};

export type CombatLabPreviewSummary = {
  battles: number;
  uniqueGovernors: number;
  killPointsGained: number;
  killPointsLost: number;
  severelyWoundedInflicted: number;
  severelyWoundedTaken: number;
  averageBattleDurationSeconds: number;
  weightedTradePercent: number;
  dps: number;
  sps: number;
  tps: number;
  hps: number;
};

export type CombatLabPreviewTrend = {
  bucketStartMs: number;
  battles: number;
  killPointsGained: number;
  killPointsLost: number;
  dps: number;
  sps: number;
  tps: number;
  hps: number;
};

export type CombatLabPreviewFormationUsagePoint = {
  bucketStartMs: number;
  sampleSize: number;
  formations: Array<{
    id: number;
    count: number;
  }>;
};

export type CombatLabPreviewArmamentSlot = {
  slot: number;
  points: CombatLabPreviewArmamentUsagePoint[];
};

export type CombatLabPreviewUsageMetric = {
  count: number;
  percent: number;
};

export type CombatLabPreviewArmamentBuff = {
  id: number;
  observations: number;
  usagePercent: number;
  averageRoll: number;
  maximumRoll: number;
  maxRollCount: number;
  maxRollPercent: number;
};

export type CombatLabPreviewArmamentUsagePoint = {
  bucketStartMs: number;
  sampleSize: number;
  inscriptions: {
    special: CombatLabPreviewUsageMetric;
    rare: CombatLabPreviewUsageMetric;
    common: CombatLabPreviewUsageMetric;
    specialCommon: CombatLabPreviewUsageMetric;
    rareCommon: CombatLabPreviewUsageMetric;
    commonCommon: CombatLabPreviewUsageMetric;
  };
  buffs: CombatLabPreviewArmamentBuff[];
};

export type CombatLabPreviewEquipmentSlot = {
  slot: number;
  points: CombatLabPreviewEquipmentUsagePoint[];
};

export type CombatLabPreviewEquipmentUsagePoint = {
  bucketStartMs: number;
  sampleSize: number;
  items: Array<{
    id: number;
    count: number;
  }>;
  legendaryCount: number;
  nonLegendaryCount: number;
  iconicLevels: Array<{
    level: number;
    count: number;
  }>;
  specialTalentCount: number;
  noSpecialTalentCount: number;
};

export type CombatLabPreviewAccessoryPairings = {
  sampleSize: number;
  pairings: Array<{
    firstItemId: number;
    secondItemId: number;
    count: number;
  }>;
};

export type CombatLabPreviewLoadouts = {
  armaments: {
    slots: CombatLabPreviewArmamentSlot[];
  };
  equipment: {
    slots: CombatLabPreviewEquipmentSlot[];
    accessoryPairings: CombatLabPreviewAccessoryPairings;
  };
};

export type CombatLabPreviewScenario = {
  summary: CombatLabPreviewSummary;
  trends: CombatLabPreviewTrend[];
  formationUsage: CombatLabPreviewFormationUsagePoint[];
  loadouts: CombatLabPreviewLoadouts;
};

export type CombatLabPreviewData = {
  generatedAtMs: number;
  pairing: {
    primaryCommanderId: number;
    primaryCommanderName: string;
    secondaryCommanderId: number;
    secondaryCommanderName: string;
  };
  drastc?: CombatLabPreviewDrastc | null;
  ranges: Partial<
    Record<
      CombatLabPreviewRangeKey,
      {
        scenarios: Partial<Record<CombatLabPreviewScenarioKey, CombatLabPreviewScenario>>;
      }
    >
  >;
};
