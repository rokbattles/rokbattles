import { memo } from "react";
import { BuildingMetric } from "@/components/territory-lab/building-metric";
import { ResourceMetric } from "@/components/territory-lab/resource-metric";
import { TerritoryBreakdown } from "@/components/territory-lab/territory-breakdown";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import type { territorySummary } from "@/lib/territory-lab/territory";
import type { Building, BuildingKind, Catalog, MapData, Plan } from "@/lib/territory-lab/types";

const BUILDINGS = [
  ["center-fortress", "Center fortress"],
  ["fortress", "Fortresses"],
  ["horse", "Alliance horse"],
  ["flag", "Flags"],
] as const;
const RESOURCES = [
  ["credits", "credits", "Alliance credits"],
  ["crystal", "crystal", "Crystal"],
  ["food", "food", "Food"],
  ["wood", "wood", "Wood"],
  ["stone", "stone", "Stone"],
  ["gold", "coin", "Gold"],
] as const;

export const TerritoryDetails = memo(function TerritoryDetails({
  plan,
  catalog,
  data,
  allianceId,
  summary,
  onLocate,
}: {
  plan: Plan;
  catalog: Catalog;
  data: MapData;
  allianceId: string;
  summary: ReturnType<typeof territorySummary>;
  onLocate: (building: Building) => void;
}) {
  const count = (kind: BuildingKind) =>
    plan.buildings.filter((b) => b.allianceId === allianceId && b.kind === kind).length;

  return (
    <div className="grid min-h-0 gap-8 overflow-hidden pt-4 xl:grid-cols-[minmax(0,1.35fr)_minmax(20rem,1fr)] xl:divide-x xl:divide-white/10">
      <TerritoryBreakdown
        entries={summary.entries}
        resources={data.resources}
        territoryOwnership={summary.ownership}
        onLocate={onLocate}
      />
      <section aria-labelledby="territory-total-heading" className="min-w-0 xl:pl-8">
        <Subheading id="territory-total-heading">Territory limits & costs</Subheading>
        <dl className="mt-4 grid grid-cols-2 gap-x-5 gap-y-2.5 text-sm">
          {BUILDINGS.filter(([kind]) => catalog.buildings[kind]).map(([kind, label]) => (
            <BuildingMetric
              key={kind}
              kind={kind}
              label={label}
              value={`${count(kind)} / ${catalog.buildings[kind]?.limit}`}
            />
          ))}
          {RESOURCES.map(([kind, icon, label]) => (
            <ResourceMetric
              key={kind}
              icon={icon}
              label={label}
              value={summary.costs[kind].toLocaleString()}
            />
          ))}
        </dl>
        {summary.costs.unknown > 0 && (
          <Text className="mt-3">
            Costs are unavailable for {summary.costs.unknown} planned buildings.
          </Text>
        )}
        <div className="mt-6 border-t pt-6 border-white/10">
          <Subheading>Territory RSS production</Subheading>
          <dl className="mt-4 grid grid-cols-2 gap-x-5 gap-y-2.5 text-sm">
            {RESOURCES.filter(([kind]) => kind !== "credits").map(([kind, icon, label]) => (
              <ResourceMetric
                key={kind}
                icon={icon}
                label={label}
                value={`${(summary.production[kind] ?? 0).toLocaleString()}/h`}
              />
            ))}
          </dl>
        </div>
      </section>
    </div>
  );
});
