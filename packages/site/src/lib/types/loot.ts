export type LootRewardAggregate = {
  type: number;
  subType: number;
  total: number;
  count: number;
};

export type PersonalLootTotals = {
  results: number;
  apUsed: number;
  honorGained: number;
  xpGained: number;
};

export type PersonalLootGroup = {
  level: number | null;
  reports: number;
  lootTotal: number;
  apUsed: number;
  honorGained: number;
  xpGained: number;
  rewards: LootRewardAggregate[];
};

export type PersonalLootQueryResult = {
  range: {
    start: string;
    end: string;
  };
  totals: PersonalLootTotals;
  groups: PersonalLootGroup[];
};
