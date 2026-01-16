"use client";

import { useTranslations } from "next-intl";
import { useContext, useMemo, useState } from "react";
import { RewardsFilters } from "@/components/my-rewards/rewards-filters";
import { Subheading } from "@/components/ui/heading";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Text } from "@/components/ui/text";
import { getLootName, getLootOrder } from "@/hooks/use-loot-name";
import { type RewardStats, useRewards } from "@/hooks/use-rewards";
import { GovernorContext } from "@/providers/governor-context";

const numberFormatter = new Intl.NumberFormat("en-US");

const DEFAULT_STATS: RewardStats = {
  totalReports: 0,
  barbKills: 0,
  barbFortKills: 0,
  otherNpcKills: 0,
};

function formatNumber(value: number): string {
  if (!Number.isFinite(value)) {
    return "0";
  }

  return numberFormatter.format(Math.round(value));
}

export function MyRewardsContent() {
  const governorContext = useContext(GovernorContext);

  if (!governorContext) {
    throw new Error("My Rewards must be used within a GovernorProvider");
  }

  const t = useTranslations("rewards");
  const { activeGovernor } = governorContext;

  const [startDate, setStartDate] = useState<string>("");
  const [endDate, setEndDate] = useState<string>("");
  const hasCustomRange = Boolean(startDate && endDate);
  const rangeStartDate = hasCustomRange ? startDate : undefined;
  const rangeEndDate = hasCustomRange ? endDate : undefined;

  const { data, loading, error } = useRewards({
    governorId: activeGovernor?.governorId,
    startDate: rangeStartDate,
    endDate: rangeEndDate,
  });

  const stats = data?.stats ?? DEFAULT_STATS;
  const summaryStats = useMemo(
    () => [
      {
        id: "barbKills",
        name: t("stats.barbs.title"),
        value: formatNumber(stats.barbKills),
      },
      {
        id: "barbForts",
        name: t("stats.forts.title"),
        value: formatNumber(stats.barbFortKills),
      },
      {
        id: "otherNpc",
        name: t("stats.other.title"),
        value: formatNumber(stats.otherNpcKills),
      },
    ],
    [stats, t]
  );

  const rewardRows = useMemo(() => {
    const rows = (data?.rewards ?? []).map((reward) => {
      const name =
        getLootName(reward.type, reward.subType) ??
        t("rewards.unknown", { type: reward.type, subType: reward.subType });

      return {
        key: `${reward.type}:${reward.subType}`,
        order: getLootOrder(reward.type, reward.subType) ?? Number.POSITIVE_INFINITY,
        name,
        total: formatNumber(reward.total),
        count: reward.count,
      };
    });

    rows.sort((a, b) => a.order - b.order);
    return rows;
  }, [data?.rewards, t]);

  if (!activeGovernor) {
    return null;
  }

  return (
    <div className="space-y-10">
      <Text>{t("intro")}</Text>
      <RewardsFilters
        startDate={startDate}
        endDate={endDate}
        onStartDateChange={setStartDate}
        onEndDateChange={setEndDate}
      />
      <section className="space-y-4">
        <Subheading>{t("stats.title")}</Subheading>
        {loading ? (
          <Text>{t("states.loading")}</Text>
        ) : error ? (
          <Text>{error}</Text>
        ) : stats.totalReports === 0 ? (
          <Text>{t("states.empty")}</Text>
        ) : (
          <div className="space-y-3">
            <div className="grid gap-6 grid-cols-2 lg:grid-cols-3">
              {summaryStats.map((stat) => (
                <div
                  key={stat.id}
                  className="space-y-3 border-b border-zinc-200/60 pb-4 dark:border-white/10"
                >
                  <div className="space-y-1">
                    <div className="text-sm font-semibold text-zinc-950 dark:text-white">
                      {stat.name}
                    </div>
                  </div>
                  <div className="mt-4 text-3xl/8 font-semibold text-zinc-950 sm:text-3xl dark:text-white">
                    {stat.value}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </section>
      <section className="space-y-4">
        <Subheading>{t("rewards.title")}</Subheading>
        {loading ? (
          <Text>{t("states.loading")}</Text>
        ) : error ? (
          <Text>{error}</Text>
        ) : rewardRows.length === 0 ? (
          <Text>{t("rewards.empty")}</Text>
        ) : (
          <Table dense className="[--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
            <TableHead>
              <TableRow>
                <TableHeader>{t("table.name")}</TableHeader>
                <TableHeader>{t("table.amount")}</TableHeader>
                <TableHeader>{t("table.drops")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {rewardRows.map((reward) => (
                <TableRow key={reward.key}>
                  <TableCell>{reward.name}</TableCell>
                  <TableCell className="tabular-nums">{reward.total}</TableCell>
                  <TableCell className="tabular-nums">{formatNumber(reward.count)}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </section>
    </div>
  );
}
