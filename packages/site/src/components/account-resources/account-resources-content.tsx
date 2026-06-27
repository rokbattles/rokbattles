"use client";

import { useSearchParams } from "next/navigation";
import { useExtracted } from "next-intl";
import { use, useMemo } from "react";
import { ResourcesErrorState } from "@/components/account-resources/resources-error-state";
import { ResourcesFiltersClient } from "@/components/account-resources/resources-filters-client";
import { ResourcesReportSection } from "@/components/account-resources/resources-report-section";
import { ResourcesTableSection } from "@/components/account-resources/resources-table-section";
import { Text } from "@/components/ui/text";
import { useResources } from "@/hooks/use-resources";
import { formatLocalDateInput } from "@/lib/datetime";
import { parseResourcesSearchParams } from "@/lib/resources/search-params";
import { GovernorContext } from "@/providers/governor-context";

type AccountResourcesContentProps = {
  datasetLocale?: string;
};

export function AccountResourcesContent({ datasetLocale }: AccountResourcesContentProps) {
  const t = useExtracted();
  const searchParams = useSearchParams();
  const governorContext = use(GovernorContext);
  if (!governorContext) {
    throw new Error("My Resources page must be used within a GovernorProvider");
  }

  const parsed = useMemo(
    () =>
      parseResourcesSearchParams({
        start: searchParams.get("start") ?? undefined,
        end: searchParams.get("end") ?? undefined,
      }),
    [searchParams]
  );

  const governorId = governorContext.activeGovernor?.governorId;
  const { data, error } = useResources({
    governorId,
    startParam: parsed.startParam,
    endParam: parsed.endParam,
  });

  if (error) {
    return <ResourcesErrorState />;
  }

  if (!data) {
    return null;
  }

  const minDate = "2025-01-01";
  const maxDate = formatLocalDateInput(new Date());

  return (
    <div className="space-y-8">
      <Text>{t("See daily gathered resources, gems, and crystals from RSS reports.")}</Text>
      <ResourcesFiltersClient
        startDate={data.range.start}
        endDate={data.range.end}
        minDate={minDate}
        maxDate={maxDate}
      />
      <section>
        <ResourcesReportSection
          totalReports={data.totalReports}
          daily={data.daily}
          rangeStart={data.range.start}
          rangeEnd={data.range.end}
          datasetLocale={datasetLocale}
        />
      </section>
      <ResourcesTableSection
        crystalsGain={data.crystalsGain}
        resources={data.resources}
        datasetLocale={datasetLocale}
      />
    </div>
  );
}
