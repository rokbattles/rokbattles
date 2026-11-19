"use client";

import { ArrowLeft, ArrowRight } from "lucide-react";
import { useSearchParams } from "next/navigation";
import { use, useEffect, useState } from "react";
import { ResourcesFiltersClient } from "@/components/account-resources/resources-filters-client";
import {
  CarriageActivityTable,
  CarriageRankings,
  CarriageTotalsChart,
} from "@/components/account-trading-post/carriage-transfer-list";
import { Button } from "@/components/ui/button";
import { Subheading } from "@/components/ui/heading";
import { Pagination } from "@/components/ui/pagination";
import { Text } from "@/components/ui/text";
import { toDateInput, todayUtcStartMillis } from "@/lib/loot/date";
import { type ResourceDateRange, resolveResourceRange } from "@/lib/resources/date-range";
import type { CarriageResponse } from "@/lib/types/carriage";
import { GovernorContext } from "@/providers/governor-context";

type RequestResult = { key: string; data: CarriageResponse | null; failed: boolean };

export function AccountCarriageTransfers() {
  const governorId = use(GovernorContext)?.activeGovernor?.governorId;
  const searchParams = useSearchParams();

  const today = toDateInput(todayUtcStartMillis());
  const range = resolveResourceRange(searchParams.get("start"), searchParams.get("end"), today);

  return governorId == null ? (
    <Text>Select a governor to view carriage transfers.</Text>
  ) : (
    <div className="space-y-6">
      <ResourcesFiltersClient
        startDate={range.start}
        endDate={range.end}
        maxDate={today}
        rangeLabel="Trading Post"
      />
      <CarriageTransfers
        key={`${governorId}:${range.start}:${range.end}`}
        governorId={governorId}
        range={range}
      />
    </div>
  );
}

function CarriageTransfers({
  governorId,
  range,
}: {
  governorId: number;
  range: ResourceDateRange;
}) {
  const [page, setPage] = useState(1);
  const [attempt, setAttempt] = useState(0);
  const [result, setResult] = useState<RequestResult | null>(null);

  const requestKey = `${page}:${attempt}`;
  const pending = result?.key !== requestKey;
  const failed = result?.key === requestKey && result.failed;
  const lastData = result?.data;

  const query = new URLSearchParams({
    page: String(page),
    start: range.start,
    end: range.end,
  }).toString();

  useEffect(() => {
    const controller = new AbortController();
    let ignore = false;

    async function loadTransfers() {
      try {
        const response = await fetch(
          `/proxy/v1/governor/${governorId}/resources/carriage?${query}`,
          { cache: "no-store", signal: controller.signal }
        );

        if (!response.ok) {
          throw new Error("Unable to load carriage transfers");
        }

        const data = (await response.json()) as CarriageResponse;

        if (ignore) {
          return;
        }

        setResult({ key: requestKey, data, failed: false });
      } catch {
        if (ignore) {
          return;
        }

        setResult((previous) => ({
          key: requestKey,
          data: previous?.data ?? null,
          failed: true,
        }));
      }
    }

    void loadTransfers();

    return () => {
      ignore = true;
      controller.abort();
    };
  }, [governorId, query, requestKey]);

  if (!lastData) {
    return (
      <div className="rounded-lg border border-zinc-800 px-6 py-12 text-center">
        {failed ? (
          <div role="alert" className="space-y-3">
            <Text>Couldn’t load carriage transfers.</Text>
            <Button outline onClick={() => setAttempt((value) => value + 1)}>
              Try again
            </Button>
          </div>
        ) : (
          <Text role="status">Loading carriage transfers…</Text>
        )}
      </div>
    );
  }

  if (lastData.totalGroups === 0) {
    return (
      <div className="rounded-md border border-dashed px-6 py-10 text-center border-zinc-700">
        <Subheading>No reports in this date range.</Subheading>
        <Text className="mt-2">
          Try another range or upload carriage reports from the desktop app.
        </Text>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <CarriageTotalsChart received={lastData.totalReceived} sent={lastData.totalSent} />
      <CarriageRankings received={lastData.topReceived} sent={lastData.topSent} />
      <section className="space-y-3" aria-labelledby="carriage-log-title" aria-busy={pending}>
        <Subheading id="carriage-log-title">Transfer log</Subheading>
        {failed ? (
          <div role="alert" className="flex flex-wrap items-center gap-3">
            <Text>Couldn’t load page {page}.</Text>
            <Button outline onClick={() => setAttempt((value) => value + 1)}>
              Try again
            </Button>
          </div>
        ) : null}
        <CarriageActivityTable activity={lastData.activity} />
        {lastData.totalPages > 1 ? (
          <Pagination aria-label="Transfer log pages" className="items-center justify-between">
            <Button
              plain
              disabled={pending || lastData.page === 1}
              onClick={() => setPage(lastData.page - 1)}
            >
              <ArrowLeft data-slot="icon" />
              Previous
            </Button>
            <Text role="status" className="tabular-nums">
              {pending ? "Loading…" : `Page ${lastData.page} of ${lastData.totalPages}`}
            </Text>
            <Button
              plain
              disabled={pending || lastData.page === lastData.totalPages}
              onClick={() => setPage(lastData.page + 1)}
            >
              Next
              <ArrowRight data-slot="icon" />
            </Button>
          </Pagination>
        ) : null}
      </section>
    </div>
  );
}
