import { useExtracted, useLocale } from "next-intl";
import { useMemo } from "react";
import { Badge } from "@/components/ui/badge";
import { clampScore } from "@/lib/combat-lab/format";

type ConfidenceScoreProps = {
  score: number;
};

type ConfidenceLevel = "veryLow" | "low" | "moderate" | "high" | "veryHigh";

export function ConfidenceScore({ score }: ConfidenceScoreProps) {
  const t = useExtracted();
  const locale = useLocale();
  const scoreFormatter = useMemo(
    () =>
      new Intl.NumberFormat(locale, {
        maximumFractionDigits: 2,
        minimumFractionDigits: 2,
      }),
    [locale]
  );
  const normalizedScore = clampScore(score);
  const percentage = Math.min(99.99, normalizedScore * 10);
  const level = getConfidenceLevel(normalizedScore);
  const levelLabel = {
    veryLow: t("Very low"),
    low: t("Low"),
    moderate: t("Moderate"),
    high: t("High"),
    veryHigh: t("Very high"),
  }[level];

  return (
    <div className="flex items-baseline justify-between gap-2">
      <div className="flex min-w-0 items-center gap-2">
        <div className="truncate font-semibold text-sm text-zinc-950 dark:text-white">
          {t("Confidence")}
        </div>
        <Badge>{t("Beta")}</Badge>
      </div>
      <div className="shrink-0 font-semibold text-sm tabular-nums text-zinc-950 dark:text-white">
        {scoreFormatter.format(percentage)}% · {levelLabel}
      </div>
    </div>
  );
}

function getConfidenceLevel(score: number): ConfidenceLevel {
  if (score < 3) {
    return "veryLow";
  }

  if (score < 5) {
    return "low";
  }

  if (score < 7) {
    return "moderate";
  }

  if (score < 9) {
    return "high";
  }

  return "veryHigh";
}
