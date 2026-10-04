import { BuildingIcon } from "@/components/territory-lab/building-icon";
import type { BuildingKind } from "@/lib/territory-lab/types";

type BuildingMetricProps = {
  kind: BuildingKind;
  label: string;
  value: string;
};

export function BuildingMetric({ kind, label, value }: BuildingMetricProps) {
  return (
    <div className="contents">
      <dt className="flex min-w-0 items-center gap-1.5 text-zinc-500 dark:text-zinc-400">
        <BuildingIcon className="size-6" kind={kind} />
        <span className="truncate">{label}</span>
      </dt>
      <dd className="text-right font-medium tabular-nums">{value}</dd>
    </div>
  );
}
