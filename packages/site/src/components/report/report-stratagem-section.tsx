"use client";

import { ChevronDown } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Subheading } from "@/components/ui/heading";
import {
  formatStratagemPercentage,
  formatStratagemStatistic,
  hasStratagems,
} from "@/lib/report/stratagems";
import type { BattleStratagem } from "@/lib/types/battle";

type ReportStratagemSectionProps = {
  stratagems?: readonly BattleStratagem[];
};

export function ReportStratagemSection({ stratagems }: ReportStratagemSectionProps) {
  if (!hasStratagems(stratagems)) {
    return null;
  }

  const statisticLabels: Record<string, string> = {
    BadHurt: "Severely wounded unit power (past 3h)",
    Atk: "Attack bonus",
    ExtraBadHurt: "Extra severe wounds inflicted",
    KillTimes: "Times triggered",
    Kill: "Enemy units killed",
    BeDmgReduceTimes: "Times triggered",
    HealTimes: "Times triggered",
    Heal: "Slightly wounded units healed",
    Dead: "Unit deaths",
    SkillDmgRaiseTimes: "Times triggered",
    SeverelyWounded: "Severe wounds prevented",
    KvkLostT5: "Tier 5 units in the Hall of Heroes",
    DamageRaise: "Damage bonus",
  };

  return (
    <div className="space-y-2">
      <Subheading>Stratagems</Subheading>
      <div className="space-y-3">
        {stratagems.map((stratagem) => (
          <details className="group" key={stratagem.id}>
            <summary className="flex cursor-pointer list-none items-start justify-between gap-3 rounded-sm focus-visible:outline-2 focus-visible:outline-blue-500 focus-visible:outline-offset-2 [&::-webkit-details-marker]:hidden">
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-1.5 font-medium text-base/6 sm:text-sm/6 text-white">
                  <span>{stratagem.name}</span>
                  {typeof stratagem.effectivePercentage === "number" ? (
                    <Badge color="emerald">
                      {`${formatStratagemPercentage(stratagem.effectivePercentage)}% applied`}
                    </Badge>
                  ) : null}
                </div>
                {stratagem.statistics.length > 0 ? (
                  <dl className="mt-1 space-y-0.5 text-base/6 sm:text-sm/6">
                    {stratagem.statistics.map((statistic) => (
                      <div className="flex gap-1" key={statistic.key}>
                        <dt className="text-zinc-400">
                          {statisticLabels[statistic.key] ?? statistic.key}:
                        </dt>
                        <dd className="font-medium tabular-nums text-zinc-200">
                          {formatStratagemStatistic(statistic)}
                        </dd>
                      </div>
                    ))}
                  </dl>
                ) : null}
              </div>
              <ChevronDown
                data-slot="icon"
                aria-hidden="true"
                className="mt-1 size-4 shrink-0 transition-transform group-open:rotate-180 text-zinc-500"
              />
            </summary>
            <p className="mt-2 whitespace-pre-line text-base/6 sm:text-sm/6 text-zinc-300">
              {stratagem.description}
            </p>
          </details>
        ))}
      </div>
    </div>
  );
}
