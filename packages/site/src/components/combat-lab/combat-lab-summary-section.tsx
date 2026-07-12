import { useExtracted } from "next-intl";
import { useMemo } from "react";
import { SummaryMetric } from "@/components/summary-metric";
import { Subheading } from "@/components/ui/heading";
import type { CombatLabPairingDocument } from "@/lib/combat-lab/api";
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
  label: string;
  value: string;
  description: string;
};

export function CombatLabSummarySection({ item }: CombatLabSummarySectionProps) {
  const t = useExtracted();
  const metrics = useMemo<CombatLabMetric[]>(
    () => [
      {
        id: "battles",
        label: t("Battles"),
        value: numberFormatter.format(item.summary.totalBattles),
        description: t("Total battle reports recorded for this pairing."),
      },
      {
        id: "kill-points-gained",
        label: t("Kill Points"),
        value: formatNumber(item.summary.killPointsGained),
        description: t("Total kill points earned while using this pairing."),
      },
      {
        id: "kill-points-lost",
        label: t("Opponent Kill Points"),
        value: formatNumber(item.summary.killPointsLost),
        description: t("Total kill points earned by opponents against this pairing."),
      },
      {
        id: "sevs-inflicted",
        label: t("Severely Wounded (Taken)"),
        value: formatNumber(item.summary.severelyWoundedTaken),
        description: t("Number of troops that became severely wounded while using this pairing."),
      },
      {
        id: "sevs-taken",
        label: t("Severely Wounded (Inflicted)"),
        value: formatNumber(item.summary.severelyWoundedInflicted),
        description: t("Number of opponent troops this pairing caused to become severely wounded."),
      },
      {
        id: "avg-duration",
        label: t("Avg. Battle Duration"),
        value: formatDuration(item.summary.avgBattleDuration),
        description: t("Average battle duration for this pairing."),
      },
      {
        id: "avg-trade",
        label: t("Avg. Trade Percentage"),
        value: formatPercent(item.summary.avgTradePercentage),
        description: t(
          "Each battle's kill points gained divided by kill points lost, then averaged across battles."
        ),
      },
      {
        id: "weighted-trade",
        label: t("Weighted Trade Percentage"),
        value: formatPercent(item.summary.weightedTradePercentage),
        description: t("Total kill points gained divided by total kill points lost."),
      },
      {
        id: "dps",
        label: t("Damage Per Second (DPS)"),
        value: formatPerSecond(item.summary.dps),
        description: t("Average damage inflicted per second while using this pairing."),
      },
      {
        id: "sps",
        label: t("Sevs Per Second (SPS)"),
        value: formatPerSecond(item.summary.sps),
        description: t(
          "Average severely wounded troops inflicted per second while using this pairing."
        ),
      },
      {
        id: "tps",
        label: t("Sevs Taken Per Second (TPS)"),
        value: formatPerSecond(item.summary.tps),
        description: t(
          "Average severely wounded troops taken per second while using this pairing."
        ),
      },
      {
        id: "hps",
        label: t("Healing Per Second (HPS)"),
        value: formatPerSecond(item.summary.hps),
        description: t("Average healing performed per second while using this pairing."),
      },
    ],
    [item, t]
  );

  return (
    <section className="space-y-4">
      <Subheading>{t("Summary")}</Subheading>
      <div className="grid grid-cols-2 gap-6 lg:grid-cols-4">
        {metrics.map((metric) => (
          <SummaryMetric
            key={metric.id}
            description={metric.description}
            label={metric.label}
            value={metric.value}
          />
        ))}
      </div>
    </section>
  );
}
