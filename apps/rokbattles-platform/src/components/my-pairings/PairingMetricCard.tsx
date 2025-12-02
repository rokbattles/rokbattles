import { Badge } from "@/components/ui/Badge";

type BadgeColor = Parameters<typeof Badge>[0] extends { color?: infer Color } ? Color : never;

export type PairingMetricCardProps = {
  label: string;
  value: number;
  previousValue?: number;
  trendDirection?: "increase" | "decrease";
  formatValue?: (value: number) => string;
  description?: string;
  comparisonLabel?: string;
};

type TrendResult = {
  color: BadgeColor;
  label: string;
};

function defaultFormatValue(value: number) {
  if (!Number.isFinite(value)) {
    return "0";
  }

  return value.toLocaleString();
}

function resolveTrend(
  value: number,
  previousValue: number | undefined,
  trendDirection: "increase" | "decrease"
): TrendResult {
  if (previousValue == null) {
    return { color: "zinc", label: "N/A" };
  }

  const safePrevious = Number.isFinite(previousValue) ? previousValue : 0;
  const safeValue = Number.isFinite(value) ? value : 0;

  if (safePrevious === 0) {
    if (safeValue === 0) {
      return { color: "zinc", label: "N/A" };
    }

    return { color: "zinc", label: "N/A" };
  }

  const delta = safeValue - safePrevious;
  const percent = (delta / safePrevious) * 100;

  if (!Number.isFinite(percent) || Math.abs(percent) < 0.05) {
    return { color: "zinc", label: "0.0%" };
  }

  const isIncrease = percent > 0;
  const prefersIncrease = trendDirection === "increase";
  const color: BadgeColor = isIncrease === prefersIncrease ? "green" : "red";
  const rounded = Math.abs(percent) >= 10 ? percent.toFixed(0) : percent.toFixed(1);
  const sign = percent > 0 ? "+" : "";

  return {
    color,
    label: `${sign}${rounded}%`,
  };
}

export function PairingMetricCard({
  label,
  value,
  previousValue,
  trendDirection = "increase",
  formatValue = defaultFormatValue,
  description,
  comparisonLabel,
}: PairingMetricCardProps) {
  const trend = resolveTrend(value, previousValue, trendDirection);

  return (
    <div className="space-y-3 border-b border-zinc-200/60 pb-4 dark:border-white/10">
      <div className="flex items-start justify-between gap-3">
        <div className="space-y-1">
          <div className="text-sm font-semibold text-zinc-950 dark:text-white">{label}</div>
          {description ? (
            <p className="text-xs text-zinc-600 dark:text-zinc-400">{description}</p>
          ) : null}
        </div>
      </div>
      <div className="mt-4 text-3xl/8 font-semibold text-zinc-950 sm:text-3xl dark:text-white">
        {formatValue(value)}
      </div>
      <div className="mt-3 flex items-center gap-2 text-xs/5 text-zinc-600 dark:text-zinc-400">
        <Badge color={trend.color}>{trend.label}</Badge>
        <span>{comparisonLabel ?? "vs previous month"}</span>
      </div>
    </div>
  );
}
