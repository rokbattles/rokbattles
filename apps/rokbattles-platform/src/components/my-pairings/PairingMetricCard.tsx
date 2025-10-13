import { Badge } from "@/components/ui/Badge";
import { Divider } from "@/components/ui/Divider";

type BadgeColor = Parameters<typeof Badge>[0] extends { color?: infer Color } ? Color : never;

export type PairingMetricCardProps = {
  label: string;
  value: number;
  previousValue?: number;
  trendDirection?: "increase" | "decrease";
  formatValue?: (value: number) => string;
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
}: PairingMetricCardProps) {
  const trend = resolveTrend(value, previousValue, trendDirection);

  return (
    <div>
      <Divider />
      <div className="mt-6 text-lg/6 font-medium sm:text-sm/6">{label}</div>
      <div className="mt-3 text-3xl/8 font-semibold sm:text-2xl/8">{formatValue(value)}</div>
      <div className="mt-3 text-sm/6 sm:text-xs/6">
        <Badge color={trend.color}>{trend.label}</Badge>{" "}
        <span className="text-zinc-500">from last month</span>
      </div>
    </div>
  );
}
