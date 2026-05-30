"use client";

import { useExtracted } from "next-intl";
import { ResourcesTimelineChartClient } from "@/components/account-resources/resources-timeline-chart-client";
import { Text } from "@/components/ui/text";
import type { ResourcesDailyAggregate } from "@/lib/types/resources";

type ResourcesReportSectionProps = {
  totalReports: number;
  daily: ResourcesDailyAggregate[];
  rangeStart: string;
  rangeEnd: string;
  datasetLocale?: string;
};

export function ResourcesReportSection({
  totalReports,
  daily,
  rangeStart,
  rangeEnd,
  datasetLocale,
}: ResourcesReportSectionProps) {
  const t = useExtracted();

  if (totalReports === 0) {
    return <Text>{t("No reports in this date range.")}</Text>;
  }

  return (
    <ResourcesTimelineChartClient
      data={daily}
      rangeStart={rangeStart}
      rangeEnd={rangeEnd}
      datasetLocale={datasetLocale}
    />
  );
}
