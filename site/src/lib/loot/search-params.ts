import "server-only";

import type { LootCategoryKey } from "@/lib/types/loot";

const DEFAULT_CATEGORY: LootCategoryKey = "barbarian";

export type LootSearchParams = Record<string, string | string[] | undefined>;

export type ParsedLootSearchParams = {
  category: LootCategoryKey;
  startParam: string | null;
  endParam: string | null;
  yearParam: string | null;
};

function firstValue(value: string | string[] | undefined): string | null {
  if (Array.isArray(value)) {
    return value[0] ?? null;
  }

  return value ?? null;
}

function parseCategory(value: string | null): LootCategoryKey {
  if (value === "barbarian" || value === "barbarianFort" || value === "baulur") {
    return value;
  }

  return DEFAULT_CATEGORY;
}

export function parseLootSearchParams(searchParams: LootSearchParams): ParsedLootSearchParams {
  const categoryValue = firstValue(searchParams.category);
  const startParam = firstValue(searchParams.start);
  const endParam = firstValue(searchParams.end);
  const yearParam = firstValue(searchParams.year);

  return {
    category: parseCategory(categoryValue),
    startParam,
    endParam,
    yearParam,
  };
}
