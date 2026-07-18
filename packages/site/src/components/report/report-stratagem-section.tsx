"use client";

import { ChevronDownIcon } from "@heroicons/react/16/solid";
import { useExtracted, useLocale } from "next-intl";
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
  const t = useExtracted();
  const locale = useLocale();

  if (!hasStratagems(stratagems)) {
    return null;
  }

  const statisticLabels: Record<string, string> = {
    BadHurt: t("Severely wounded unit power (past 3h)"),
    Atk: t("Attack bonus"),
    ExtraBadHurt: t("Extra severe wounds inflicted"),
    KillTimes: t("Times triggered"),
    Kill: t("Enemy units killed"),
    BeDmgReduceTimes: t("Times triggered"),
    HealTimes: t("Times triggered"),
    Heal: t("Slightly wounded units healed"),
    Dead: t("Unit deaths"),
    SkillDmgRaiseTimes: t("Times triggered"),
    SeverelyWounded: t("Severe wounds prevented"),
    KvkLostT5: t("Tier 5 units in the Hall of Heroes"),
    DamageRaise: t("Damage bonus"),
  };

  return (
    <div className="space-y-2">
      <Subheading>{t("Stratagems")}</Subheading>
      <div className="space-y-3">
        {stratagems.map((stratagem) => (
          <details className="group" key={stratagem.id}>
            <summary className="flex cursor-pointer list-none items-start justify-between gap-3 rounded-sm focus-visible:outline-2 focus-visible:outline-blue-500 focus-visible:outline-offset-2 [&::-webkit-details-marker]:hidden">
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-1.5 font-medium text-base/6 text-zinc-950 sm:text-sm/6 dark:text-white">
                  <span>{stratagem.name}</span>
                  {typeof stratagem.effectivePercentage === "number" ? (
                    <Badge color="emerald">
                      {t("{percentage}% applied", {
                        percentage: formatStratagemPercentage(
                          stratagem.effectivePercentage,
                          locale
                        ),
                      })}
                    </Badge>
                  ) : null}
                </div>
                {stratagem.statistics.length > 0 ? (
                  <dl className="mt-1 space-y-0.5 text-base/6 sm:text-sm/6">
                    {stratagem.statistics.map((statistic) => (
                      <div className="flex gap-1" key={statistic.key}>
                        <dt className="text-zinc-500 dark:text-zinc-400">
                          {statisticLabels[statistic.key] ?? statistic.key}:
                        </dt>
                        <dd className="font-medium tabular-nums text-zinc-800 dark:text-zinc-200">
                          {formatStratagemStatistic(statistic, locale)}
                        </dd>
                      </div>
                    ))}
                  </dl>
                ) : null}
              </div>
              <ChevronDownIcon className="mt-1 size-4 shrink-0 fill-zinc-400 transition-transform group-open:rotate-180 dark:fill-zinc-500" />
            </summary>
            <p className="mt-2 whitespace-pre-line text-base/6 text-zinc-600 sm:text-sm/6 dark:text-zinc-300">
              {stratagem.description}
            </p>
          </details>
        ))}
      </div>
    </div>
  );
}
