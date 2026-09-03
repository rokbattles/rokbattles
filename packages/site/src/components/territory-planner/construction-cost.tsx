"use client";

import { useExtracted } from "next-intl";
import { ResourceIcon } from "@/components/territory-planner/resource-icon";
import type { BuildingCost, CostResourceKind, ResourceKind } from "@/lib/territory/types";

const COST_FIELDS: Array<{
  field: CostResourceKind;
  icon: ResourceKind | "credits";
}> = [
  { field: "credits", icon: "credits" },
  { field: "crystal", icon: "crystal" },
  { field: "food", icon: "food" },
  { field: "wood", icon: "wood" },
  { field: "stone", icon: "stone" },
  { field: "gold", icon: "coin" },
];

type ConstructionCostProps = {
  cost: BuildingCost | null;
  labels: Record<CostResourceKind, string>;
  numberFormatter: Intl.NumberFormat;
};

export function ConstructionCost({ cost, labels, numberFormatter }: ConstructionCostProps) {
  const t = useExtracted();

  if (!cost) {
    return (
      <p className="text-xs text-amber-700 dark:text-amber-400">
        {t("No construction cost is available")}
      </p>
    );
  }
  const values = COST_FIELDS.filter(({ field }) => (cost[field] ?? 0) > 0);
  if (values.length === 0) {
    return <p className="text-xs text-zinc-500 dark:text-zinc-400">{t("No cost")}</p>;
  }
  return (
    <ul aria-label={t("Construction cost")} className="flex flex-wrap gap-x-3 gap-y-1">
      {values.map(({ field, icon }) => {
        const label = labels[field];
        return (
          <li
            className="flex items-center gap-1 text-xs text-zinc-600 dark:text-zinc-300"
            key={field}
            title={label}
          >
            <ResourceIcon kind={icon} />
            <span className="sr-only">{label}: </span>
            <span className="font-medium tabular-nums">
              {numberFormatter.format(cost[field] ?? 0)}
            </span>
          </li>
        );
      })}
    </ul>
  );
}
