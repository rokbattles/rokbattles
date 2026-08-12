import type { ReactNode } from "react";

type SummaryMetricProps = {
  description: string;
  label: ReactNode;
  value: string;
};

export function SummaryMetric({ description, label, value }: SummaryMetricProps) {
  return (
    <div className="flex h-full flex-col border-zinc-200/60 border-b pb-4 dark:border-white/10">
      <div className="space-y-1">
        <div className="font-semibold text-sm text-zinc-950 dark:text-white">{label}</div>
        <p className="text-sm text-zinc-600 dark:text-zinc-400">{description}</p>
      </div>
      <div className="mt-auto pt-3 font-semibold text-2xl/8 text-zinc-950 dark:text-white">
        {value}
      </div>
    </div>
  );
}
