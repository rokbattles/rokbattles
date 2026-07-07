import { useExtracted } from "next-intl";
import { useMemo } from "react";
import { DrastcRadarChart } from "@/components/combat-lab/drastc-radar-chart";
import type { CombatLabDrastcScore } from "@/lib/combat-lab/api";
import type { DrastcRadarDatum } from "@/lib/combat-lab/chart";

type DrastcRadarProps = {
  score: CombatLabDrastcScore;
};

export function DrastcRadar({ score }: DrastcRadarProps) {
  const t = useExtracted();
  const chartData = useMemo<DrastcRadarDatum[]>(
    () => [
      { axis: "D", fullName: t("Damage"), score: score.breakdown.damage.score },
      { axis: "R", fullName: t("Rage/Skill Cycle Efficiency"), score: score.breakdown.rage.score },
      { axis: "A", fullName: t("Assist/Support"), score: score.breakdown.assist.score },
      { axis: "S", fullName: t("Sustainability"), score: score.breakdown.sustainability.score },
      { axis: "T", fullName: t("Trade Efficiency"), score: score.breakdown.trade.score },
      { axis: "C", fullName: t("Consistency"), score: score.breakdown.consistency.score },
    ],
    [score, t]
  );

  return <DrastcRadarChart data={chartData} />;
}
