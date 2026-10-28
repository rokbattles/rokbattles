"use client";

import { type ReactElement, useMemo } from "react";
import { ResourcesBreakdownTable } from "@/components/account-resources/resources-breakdown-table";
import { ResourcesLedgerClient } from "@/components/account-resources/resources-ledger-client";
import { ResourcesSummaryClient } from "@/components/account-resources/resources-summary-client";
import { ResourcesTimelineChartClient } from "@/components/account-resources/resources-timeline-chart-client";
import { Button } from "@/components/ui/button";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import { buildResourceDays } from "@/lib/resources/analytics";
import type { ResourceKey } from "@/lib/resources/catalog";
import { buildResourceBreakdownRows } from "@/lib/resources/rows";
import type { ResourcesQueryResult } from "@/lib/types/resources";

export type ResourceView = {
  selected: ResourceKey[];
  cumulative: boolean;
};

type ResourcesDashboardClientProps = {
  data: ResourcesQueryResult;
  governorId: number;
  today: string;
  view: ResourceView;
  onViewChange: (patch: Partial<ResourceView>) => void;
};

export function ResourcesDashboardClient({
  data,
  governorId,
  today,
  view,
  onViewChange,
}: ResourcesDashboardClientProps): ReactElement {
  const { selected, cumulative } = view;
  const rows = useMemo(
    () => buildResourceBreakdownRows(data.crystalsGain, data.resources),
    [data.crystalsGain, data.resources]
  );
  const days = useMemo(
    () => buildResourceDays(data.daily, data.range.start, data.range.end, today),
    [data, today]
  );
  const visibleRows = rows.filter((row) => selected.includes(row.key));

  const toggleResource = (key: ResourceKey): void => {
    onViewChange({
      selected: selected.includes(key)
        ? selected.filter((item) => item !== key)
        : [...selected, key],
    });
  };

  let report: ReactElement;

  if (data.totalReports === 0) {
    report = (
      <div className="rounded-md border border-dashed px-6 py-10 text-center border-zinc-700">
        <Subheading>No reports in this date range.</Subheading>
        <Text className="mt-2">
          Try another range or upload gathering reports from the desktop app.
        </Text>
      </div>
    );
  } else if (selected.length === 0) {
    report = <Text>Select a resource above to view its trends and ledger.</Text>;
  } else {
    report = (
      <>
        <section className="space-y-4">
          <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
            <div className="min-w-0">
              <Subheading>Gathering trends</Subheading>
              <Text className="mt-1">
                {cumulative
                  ? "Running totals start at the beginning of your selected range. Each chart has its own scale."
                  : "Recorded daily gains, including bonuses. Each chart has its own scale."}
              </Text>
            </div>
            <div
              className="flex shrink-0 self-start gap-1 rounded-lg p-1 bg-zinc-900"
              role="group"
              aria-label="Chart values"
            >
              <Button
                {...(!cumulative ? { color: "dark" } : { plain: true })}
                aria-pressed={!cumulative}
                onClick={() => onViewChange({ cumulative: false })}
              >
                Periodic
              </Button>
              <Button
                {...(cumulative ? { color: "dark" } : { plain: true })}
                aria-pressed={cumulative}
                onClick={() => onViewChange({ cumulative: true })}
              >
                Cumulative
              </Button>
            </div>
          </div>
          <ResourcesTimelineChartClient days={days} rows={visibleRows} cumulative={cumulative} />
        </section>
        <ResourcesBreakdownTable rows={visibleRows} />
        <ResourcesLedgerClient days={days} rows={visibleRows} governorId={governorId} />
      </>
    );
  }

  return (
    <div className="space-y-8">
      <section className="space-y-4">
        <div>
          <Subheading>Gathering overview</Subheading>
          <Text className="mt-1">
            Select cards to filter charts and tables. Averages include every day in the range.
          </Text>
        </div>
        <ResourcesSummaryClient
          rows={rows}
          selected={selected}
          days={days.length}
          onToggle={toggleResource}
        />
      </section>
      {report}
    </div>
  );
}
