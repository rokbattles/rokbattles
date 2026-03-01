import { getLootName } from "@/lib/loot-catalog";
import { getResourceName, RESOURCE_TYPE_IDS } from "@/lib/resources/catalog";
import type { ResourceTotals, ResourceTotalsByType } from "@/lib/types/resources";

export type ResourceBreakdownRow = {
  key: string;
  name: string;
  gain: number;
  bonus: number;
  total: number;
};

export function buildResourceBreakdownRows(
  crystalsGain: ResourceTotals,
  resources: ResourceTotalsByType[],
  locale?: string
): ResourceBreakdownRow[] {
  const crystalsName = getLootName(1, 9, locale);
  if (!crystalsName) {
    throw new Error("Missing crystals name in loot dataset");
  }

  const resourcesByType = new Map(resources.map((resource) => [resource.type, resource]));

  const rows: ResourceBreakdownRow[] = [
    {
      key: "crystalsGain",
      name: crystalsName,
      gain: crystalsGain.gain,
      bonus: crystalsGain.bonus,
      total: crystalsGain.total,
    },
  ];

  for (const type of RESOURCE_TYPE_IDS) {
    const resourceName = getResourceName(type, locale);
    if (!resourceName) {
      throw new Error(`Missing resource type ${type} in resources dataset`);
    }

    const resource = resourcesByType.get(type);
    rows.push({
      key: `type:${type}`,
      name: resourceName,
      gain: resource?.gain ?? 0,
      bonus: resource?.bonus ?? 0,
      total: resource?.total ?? 0,
    });
  }

  return rows;
}
