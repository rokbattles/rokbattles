import { getLootName, getLootSprites } from "@/lib/loot-catalog";
import { RESOURCE_CATALOG, type ResourceKey } from "@/lib/resources/catalog";
import type { ResourceTotals, ResourceTotalsByType } from "@/lib/types/resources";

export type ResourceBreakdownRow = ResourceTotals & {
  key: ResourceKey;
  name: string;
  color: string;
  spriteUrls?: string[];
};

export function buildResourceBreakdownRows(
  crystalsGain: ResourceTotals,
  resources: ResourceTotalsByType[],
  locale?: string
): ResourceBreakdownRow[] {
  const resourcesByType = new Map(resources.map((resource) => [resource.type, resource]));

  return RESOURCE_CATALOG.map(({ key, type, color }) => {
    const name = getLootName(1, type, locale);

    if (!name) {
      throw new Error(`Missing resource subtype ${type} in loot dataset type 1`);
    }

    const resource = key === "crystalsGain" ? crystalsGain : resourcesByType.get(type);

    return {
      key,
      name,
      color,
      spriteUrls: getLootSprites(1, type),
      gain: resource?.gain ?? 0,
      bonus: resource?.bonus ?? 0,
      total: resource?.total ?? 0,
    };
  });
}
