"use client";

import { useExtracted } from "next-intl";
import { LootTimelineChartClient } from "@/components/account-loot/loot-timeline-chart-client";
import { Text } from "@/components/ui/text";
import type { LootCategoryAggregate } from "@/lib/types/loot";

type LootReportSectionProps = {
  category: LootCategoryAggregate;
  rangeStart: string;
  rangeEnd: string;
};

export function LootReportSection({ category, rangeStart, rangeEnd }: LootReportSectionProps) {
  const t = useExtracted();

  if (category.reports === 0) {
    return <Text>{t("No reports in this date range.")}</Text>;
  }

  return (
    <LootTimelineChartClient data={category.daily} rangeStart={rangeStart} rangeEnd={rangeEnd} />
  );
}
