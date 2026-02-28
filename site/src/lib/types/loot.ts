export type LootCategoryKey = "barbarian" | "barbarianFort" | "baulur";

export type LootRewardAggregate = {
  type: number;
  subType: number;
  total: number;
  count: number;
};

export type LootDailyAggregate = {
  date: string;
  reports: number;
  lootTotal: number;
};

export type LootCategoryAggregate = {
  reports: number;
  lootTotal: number;
  rewards: LootRewardAggregate[];
  daily: LootDailyAggregate[];
};

export type LootQueryInput = {
  governorId: number;
  startParam?: string | null;
  endParam?: string | null;
};

export type LootQueryResult = {
  range: {
    start: string;
    end: string;
  };
  totalReports: number;
  categories: Record<LootCategoryKey, LootCategoryAggregate>;
};
