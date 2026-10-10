"use client";

import { useSearchParams } from "next/navigation";
import { useExtracted } from "next-intl";
import { type ReactElement, use, useState } from "react";
import {
  ResourcesDashboardClient,
  type ResourceView,
} from "@/components/account-resources/resources-dashboard-client";
import { ResourcesFiltersClient } from "@/components/account-resources/resources-filters-client";
import { Button } from "@/components/ui/button";
import { Text } from "@/components/ui/text";
import { useResources } from "@/hooks/use-resources";
import { toDateInput, todayUtcStartMillis } from "@/lib/loot/date";
import { RESOURCE_KEYS } from "@/lib/resources/catalog";
import { resolveResourceRange } from "@/lib/resources/date-range";
import { GovernorContext } from "@/providers/governor-context";

type AccountResourcesContentProps = {
  datasetLocale?: string;
};

export function AccountResourcesContent({
  datasetLocale,
}: AccountResourcesContentProps): ReactElement {
  const t = useExtracted();
  const searchParams = useSearchParams();
  const governorContext = use(GovernorContext);
  const [view, setView] = useState<ResourceView>({
    selected: [...RESOURCE_KEYS],
    cumulative: false,
  });

  if (!governorContext) {
    throw new Error("My Resources page must be used within a GovernorProvider");
  }

  const today = toDateInput(todayUtcStartMillis());
  const range = resolveResourceRange(searchParams.get("start"), searchParams.get("end"), today);
  const governorId = governorContext.activeGovernor?.governorId;
  const { data, error, loading, retry } = useResources({
    governorId,
    startParam: range.start,
    endParam: range.end,
  });

  let content: ReactElement;

  if (loading) {
    content = (
      <div className="rounded-md border px-6 py-16 text-center border-zinc-800" role="status">
        <Text>{t("Loading gathering reports…")}</Text>
      </div>
    );
  } else if (error) {
    content = (
      <div role="alert" className="space-y-3">
        <Text>{t("Failed to load resources.")}</Text>
        <Button outline onClick={retry}>
          {t("Try again")}
        </Button>
      </div>
    );
  } else if (data) {
    content = (
      <ResourcesDashboardClient
        key={`${governorId}:${range.start}:${range.end}`}
        view={view}
        onViewChange={(patch) => setView((current) => ({ ...current, ...patch }))}
        data={data}
        governorId={governorId}
        today={today}
        datasetLocale={datasetLocale}
      />
    );
  } else {
    content = <Text>{t("Select a governor to view gathering reports.")}</Text>;
  }

  return (
    <div className="mt-4 space-y-6">
      <Text>
        {t("Track your gathering, understand your bonuses, and see how each resource adds up.")}
      </Text>
      <ResourcesFiltersClient startDate={range.start} endDate={range.end} maxDate={today} />
      {content}
    </div>
  );
}
