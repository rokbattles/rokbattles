import "server-only";

import { parseNumeric } from "@/data/loot/normalizers";
import type { LootCategoryAggregate, LootRewardAggregate } from "@/lib/types/loot";

type LootEntryDocument = {
  type?: unknown;
  sub_type?: unknown;
  value?: unknown;
};

type DailyBucket = {
  date: string;
  reports: number;
  lootTotal: number;
};

type CategoryAggregateInternal = {
  reports: number;
  lootTotal: number;
  rewardBuckets: Map<string, LootRewardAggregate>;
  dailyBuckets: Map<string, DailyBucket>;
};

function getDailyBucket(category: CategoryAggregateInternal, dateKey: string): DailyBucket {
  const existing = category.dailyBuckets.get(dateKey);
  if (existing) {
    return existing;
  }

  const created: DailyBucket = {
    date: dateKey,
    reports: 0,
    lootTotal: 0,
  };
  category.dailyBuckets.set(dateKey, created);
  return created;
}

export function createCategoryAggregate(): CategoryAggregateInternal {
  return {
    reports: 0,
    lootTotal: 0,
    rewardBuckets: new Map(),
    dailyBuckets: new Map(),
  };
}

export function addReport(category: CategoryAggregateInternal, dateKey: string): void {
  category.reports += 1;
  const dailyBucket = getDailyBucket(category, dateKey);
  dailyBucket.reports += 1;
}

export function addLoot(
  category: CategoryAggregateInternal,
  dateKey: string,
  lootEntries: LootEntryDocument[] | null | undefined
): void {
  if (!Array.isArray(lootEntries) || lootEntries.length === 0) {
    return;
  }

  const dailyBucket = getDailyBucket(category, dateKey);

  for (const entry of lootEntries) {
    const type = parseNumeric(entry.type);
    const subType = parseNumeric(entry.sub_type);
    const value = parseNumeric(entry.value);

    if (type == null || subType == null || value == null) {
      continue;
    }

    category.lootTotal += value;
    dailyBucket.lootTotal += value;

    const key = `${type}:${subType}`;
    const bucket = category.rewardBuckets.get(key);
    if (bucket) {
      bucket.total += value;
      bucket.count += 1;
      continue;
    }

    category.rewardBuckets.set(key, {
      type,
      subType,
      total: value,
      count: 1,
    });
  }
}

export function toCategoryPayload(category: CategoryAggregateInternal): LootCategoryAggregate {
  return {
    reports: category.reports,
    lootTotal: category.lootTotal,
    rewards: Array.from(category.rewardBuckets.values()),
    daily: Array.from(category.dailyBuckets.values()).sort((a, b) => a.date.localeCompare(b.date)),
  };
}
