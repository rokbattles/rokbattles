import { getExtracted } from "next-intl/server";
import { LootTimelineChartClient } from "@/components/account-loot/loot-timeline-chart-client";
import { Text } from "@/components/ui/text";
import type { LootCategoryAggregate } from "@/lib/types/loot";

type LootReportSectionProps = {
  category: LootCategoryAggregate;
  rangeStart: string;
  rangeEnd: string;
};

export async function LootReportSection({
  category,
  rangeStart,
  rangeEnd,
}: LootReportSectionProps) {
  const t = await getExtracted();

  if (category.reports === 0) {
    return <Text>{t("No reports in this date range.")}</Text>;
  }

  return (
    <LootTimelineChartClient data={category.daily} rangeStart={rangeStart} rangeEnd={rangeEnd} />
  );
}
