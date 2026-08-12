import { InformationCircleIcon } from "@heroicons/react/16/solid";
import { useExtracted } from "next-intl";
import { type ReactNode, useMemo, useState } from "react";
import { FormationBreakdown } from "@/components/combat-lab/formation-breakdown";
import { SummaryMetric } from "@/components/summary-metric";
import { Subheading } from "@/components/ui/heading";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/listbox";
import { GameTranslate } from "@/components/v1/game-translate";
import type { CombatLabPairingDocument, CombatLabStrategies } from "@/lib/combat-lab/api";
import {
  formatDuration,
  formatNumber,
  formatPercent,
  formatPerSecond,
  numberFormatter,
} from "@/lib/combat-lab/format";

type CombatLabSummarySectionProps = {
  item: CombatLabPairingDocument;
};

type CombatLabMetric = {
  id: string;
  label: ReactNode;
  value: string;
  description: string;
};

type CombatLabStrategy = keyof CombatLabStrategies;

export function CombatLabSummarySection({ item }: CombatLabSummarySectionProps) {
  const t = useExtracted();
  const [strategy, setStrategy] = useState<CombatLabStrategy>("openField");
  const summary = item.strategies[strategy];
  const metrics = useMemo<CombatLabMetric[]>(
    () => [
      {
        id: "battles",
        label: t("Battles"),
        value: numberFormatter.format(summary.totalBattles),
        description: t("Total battle reports recorded for this pairing."),
      },
      {
        id: "kill-points-gained",
        label: <GameTranslate value="LC_COMMON_KILL_SCORE" />,
        value: formatNumber(summary.killPointsGained),
        description: t("Total kill points earned while using this pairing."),
      },
      {
        id: "kill-points-lost",
        label: t("Opponent Kill Points"),
        value: formatNumber(summary.killPointsLost),
        description: t("Total kill points earned by opponents against this pairing."),
      },
      {
        id: "sevs-inflicted",
        label: t("Severely Wounded (Taken)"),
        value: formatNumber(summary.severelyWoundedTaken),
        description: t("Number of troops that became severely wounded while using this pairing."),
      },
      {
        id: "sevs-taken",
        label: t("Severely Wounded (Inflicted)"),
        value: formatNumber(summary.severelyWoundedInflicted),
        description: t("Number of opponent troops this pairing caused to become severely wounded."),
      },
      {
        id: "avg-duration",
        label: t("Avg. Battle Duration"),
        value: formatDuration(summary.avgBattleDuration),
        description: t("Average battle duration for this pairing."),
      },
      {
        id: "avg-trade",
        label: t("Avg. Trade Percentage"),
        value: formatPercent(summary.avgTradePercentage),
        description: t(
          "Each battle's kill points gained divided by kill points lost, then averaged across battles."
        ),
      },
      {
        id: "weighted-trade",
        label: t("Weighted Trade Percentage"),
        value: formatPercent(summary.weightedTradePercentage),
        description: t("Total kill points gained divided by total kill points lost."),
      },
      {
        id: "dps",
        label: t("Damage Per Second (DPS)"),
        value: formatPerSecond(summary.dps),
        description: t("Average damage inflicted per second while using this pairing."),
      },
      {
        id: "sps",
        label: t("Sevs Per Second (SPS)"),
        value: formatPerSecond(summary.sps),
        description: t(
          "Average severely wounded troops inflicted per second while using this pairing."
        ),
      },
      {
        id: "tps",
        label: t("Sevs Taken Per Second (TPS)"),
        value: formatPerSecond(summary.tps),
        description: t(
          "Average severely wounded troops taken per second while using this pairing."
        ),
      },
      {
        id: "hps",
        label: t("Healing Per Second (HPS)"),
        value: formatPerSecond(summary.hps),
        description: t("Average healing performed per second while using this pairing."),
      },
    ],
    [summary, t]
  );

  return (
    <section className="space-y-4">
      <Subheading>{t("Summary")}</Subheading>
      <div className="grid grid-cols-2 gap-x-6 gap-y-2 lg:grid-cols-4">
        <Listbox<CombatLabStrategy>
          aria-label={t("Summary strategy")}
          onChange={setStrategy}
          value={strategy}
        >
          <ListboxOption value="all">
            <ListboxLabel>{t("All")}</ListboxLabel>
          </ListboxOption>
          <ListboxOption value="openField">
            <ListboxLabel>{t("Open Field")}</ListboxLabel>
          </ListboxOption>
          <ListboxOption value="swarming">
            <ListboxLabel>{t("Swarming")}</ListboxLabel>
          </ListboxOption>
          <ListboxOption value="rally">
            <ListboxLabel>{t("Rally")}</ListboxLabel>
          </ListboxOption>
          <ListboxOption value="garrison">
            <ListboxLabel>{t("Garrison")}</ListboxLabel>
          </ListboxOption>
        </Listbox>
        {strategy === "openField" ? null : (
          <div
            className="col-span-full flex gap-2 rounded-lg border border-blue-200 bg-blue-50 px-3 py-2 text-blue-800 dark:border-blue-900 dark:bg-blue-950/40 dark:text-blue-300"
            role="status"
          >
            <InformationCircleIcon aria-hidden="true" className="mt-0.5 size-5 shrink-0" />
            <p className="text-sm/6">
              {t(
                "The DRASTC score above is calculated from Open Field battles only. Changing the summary view below does not affect the score."
              )}
            </p>
          </div>
        )}
      </div>
      <div className="grid grid-cols-2 gap-6 lg:grid-cols-4">
        {metrics.map((metric) => (
          <SummaryMetric
            key={metric.id}
            description={metric.description}
            label={metric.label}
            value={metric.value}
          />
        ))}
        <FormationBreakdown formations={summary.formations} />
      </div>
    </section>
  );
}
