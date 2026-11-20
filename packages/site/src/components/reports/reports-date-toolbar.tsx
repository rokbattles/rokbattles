"use client";

import { ResourcesFiltersClient } from "@/components/account-resources/resources-filters-client";
import { useReportsRange } from "@/hooks/use-reports-range";

/** The same calendar and quick ranges as the Pairings page, in the same spot under the page title. */
export function ReportsDateToolbar() {
  const { range, today } = useReportsRange();

  return (
    <ResourcesFiltersClient
      startDate={range.start}
      endDate={range.end}
      maxDate={today}
      rangeLabel="Battle report"
    />
  );
}
