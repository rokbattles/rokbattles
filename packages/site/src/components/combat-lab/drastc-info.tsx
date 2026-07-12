import { useExtracted } from "next-intl";
import { BreakdownScoreBar } from "@/components/combat-lab/breakdown-score-bar";
import { DrastcCredits } from "@/components/combat-lab/drastc-credits";
import { Subheading } from "@/components/ui/heading";
import type { CombatLabCategoryScore, CombatLabDrastcScore } from "@/lib/combat-lab/api";

type DrastcInfoProps = {
  score: CombatLabDrastcScore;
};

type DrastcInfoRow = {
  key: string;
  label: string;
  metric: CombatLabCategoryScore;
};

export function DrastcInfo({ score }: DrastcInfoProps) {
  const t = useExtracted();
  const rows: DrastcInfoRow[] = [
    {
      key: "D",
      label: t("Damage"),
      metric: score.breakdown.damage,
    },
    {
      key: "R",
      label: t("Rage/Skill Cycle Efficiency"),
      metric: score.breakdown.rage,
    },
    {
      key: "A",
      label: t("Assist/Support"),
      metric: score.breakdown.assist,
    },
    {
      key: "S",
      label: t("Sustainability"),
      metric: score.breakdown.sustainability,
    },
    {
      key: "T",
      label: t("Trade Efficiency"),
      metric: score.breakdown.trade,
    },
    {
      key: "C",
      label: t("Consistency"),
      metric: score.breakdown.consistency,
    },
  ];

  return (
    <div className="w-full self-start space-y-6 text-left">
      <Subheading>{t("DRASTC breakdown")}</Subheading>
      <div className="space-y-8">
        <div className="grid gap-4 lg:grid-cols-2">
          {rows.map((row) => (
            <BreakdownScoreBar
              key={row.key}
              badge={row.key}
              label={row.label}
              score={row.metric.score}
            />
          ))}
        </div>
        <DrastcCredits />
      </div>
    </div>
  );
}
