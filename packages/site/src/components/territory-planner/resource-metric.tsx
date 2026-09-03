import { ResourceIcon } from "@/components/territory-planner/resource-icon";
import type { ResourceKind } from "@/lib/territory/types";

type ResourceMetricProps = {
  icon: ResourceKind | "credits";
  label: string;
  value: string;
};

export function ResourceMetric({ icon, label, value }: ResourceMetricProps) {
  return (
    <div className="contents">
      <dt className="flex min-w-0 items-center gap-1.5 text-zinc-500 dark:text-zinc-400">
        <ResourceIcon kind={icon} />
        <span className="truncate">{label}</span>
      </dt>
      <dd className="text-right font-medium tabular-nums">{value}</dd>
    </div>
  );
}
