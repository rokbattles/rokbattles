import { getLootName, getLootOrder, getLootSprites } from "@/lib/loot-catalog";
import type { LootCategoryAggregate } from "@/lib/types/loot";

export type LootRewardRow = {
  key: string;
  order: number;
  name: string;
  spriteUrls?: string[];
  total: number;
  count: number;
};

export function buildLootRewardRows(
  category: LootCategoryAggregate,
  fallbackName: (type: number, subType: number) => string,
  locale?: string
): LootRewardRow[] {
  const rows: LootRewardRow[] = category.rewards.map((reward) => {
    const name =
      getLootName(reward.type, reward.subType, locale) ?? fallbackName(reward.type, reward.subType);

    return {
      key: `${reward.type}:${reward.subType}`,
      order: getLootOrder(reward.type, reward.subType) ?? Number.POSITIVE_INFINITY,
      name,
      spriteUrls: getLootSprites(reward.type, reward.subType),
      total: reward.total,
      count: reward.count,
    };
  });

  rows.sort((a, b) => a.order - b.order || b.total - a.total);
  return rows;
}
