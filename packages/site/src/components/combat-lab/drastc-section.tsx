import { useExtracted } from "next-intl";
import { BreakdownScoreBar } from "@/components/combat-lab/breakdown-score-bar";
import { DrastcInfo } from "@/components/combat-lab/drastc-info";
import { DrastcRadar } from "@/components/combat-lab/drastc-radar";
import { Subheading } from "@/components/ui/heading";
import type { CombatLabDrastcScore } from "@/lib/combat-lab/api";

type DrastcSectionProps = {
  score: CombatLabDrastcScore;
};

export function DrastcSection({ score }: DrastcSectionProps) {
  const t = useExtracted();

  return (
    <section>
      <div className="space-y-6 lg:flex lg:items-start lg:gap-8 lg:space-y-0">
        <div className="space-y-4 lg:w-[28rem] lg:flex-none">
          <Subheading>{t("DRASTC scoring")}</Subheading>
          <div className="space-y-4">
            <DrastcRadar score={score} />
            <BreakdownScoreBar label={t("Overall score")} score={score.overall} />
          </div>
        </div>
        <DrastcInfo score={score} />
      </div>
    </section>
  );
}
