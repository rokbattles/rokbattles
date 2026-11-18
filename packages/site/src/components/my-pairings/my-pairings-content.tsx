"use client";

import { ArrowDown, ArrowUp, ChevronRight } from "lucide-react";
import { useMemo, useState } from "react";
import { PairingsCharts } from "@/components/my-pairings/pairings-charts";
import {
  PairingIdentity,
  PairingsState,
  PairingsToolbar,
  usePairingsContext,
} from "@/components/my-pairings/pairings-shared";
import { Heading, Subheading } from "@/components/ui/heading";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { usePairingsQuery } from "@/hooks/use-pairings";
import type { PairingAggregate, PairingsResponse } from "@/lib/pairings";
import {
  formatCompact,
  formatExact,
  formatTrade,
  MIN_PAIRING_BATTLES,
  netKp,
  pairingKey,
  pairingName,
  tradeRatio,
} from "@/lib/pairings/analytics";

type SortKey = "count" | "gained" | "lost" | "net" | "trade";

const sortValue = (item: PairingAggregate, key: SortKey) =>
  ({
    count: item.count,
    gained: item.totals.killScore,
    lost: item.totals.enemyKillScore,
    net: netKp(item.totals),
    trade: tradeRatio(item.totals) ?? (item.totals.killScore > 0 ? Number.POSITIVE_INFINITY : -1),
  })[key];

export function MyPairingsContent() {
  const { governorId, query, range, today } = usePairingsContext();
  const result = usePairingsQuery<PairingsResponse>(
    governorId ? `/proxy/v1/governor/${governorId}/pairings?${query}` : null
  );

  const items = result.data?.items.filter((item) => item.count >= MIN_PAIRING_BATTLES) ?? [];

  return (
    <>
      <Heading>My Pairings</Heading>
      <div className="relative mt-4 min-w-0 max-w-full space-y-8 overflow-x-clip">
        <PairingsToolbar range={range} today={today} />
        {result.loading ? (
          <PairingsState loading />
        ) : result.error ? (
          <PairingsState error={result.error} retry={result.retry} />
        ) : !items.length ? (
          <PairingsState>No pairings with 100+ battles in this range.</PairingsState>
        ) : (
          <PairingsOverview key={governorId} items={items} query={query.toString()} />
        )}
      </div>
    </>
  );
}

function PairingsOverview({ items, query }: { items: PairingAggregate[]; query: string }) {
  return (
    <>
      <PairingsCharts items={items} query={query} />
      <PairingsTable items={items} query={query} />
    </>
  );
}

function PairingsTable({ items, query }: { items: PairingAggregate[]; query: string }) {
  const [sort, setSort] = useState<{ key: SortKey; descending: boolean }>({
    key: "net",
    descending: true,
  });

  const rows = useMemo(
    () =>
      items.toSorted((a, b) => {
        const left = sortValue(a, sort.key),
          right = sortValue(b, sort.key);
        return left === right
          ? b.count - a.count
          : (left > right ? 1 : -1) * (sort.descending ? -1 : 1);
      }),
    [items, sort]
  );

  return (
    <section className="space-y-4">
      <Subheading>Pairing overview</Subheading>
      <Table
        dense
        className="[--gutter:--spacing(4)] [&>div]:w-full [&_table]:w-full [&_table]:min-w-[48rem]"
      >
        <caption className="sr-only">Commander pairings with at least 100 battles</caption>
        <TableHead>
          <TableRow>
            <TableHeader>Pairing</TableHeader>
            {(
              [
                ["count", "Battles"],
                ["gained", "KP gained"],
                ["lost", "KP lost"],
                ["net", "Net KP"],
                ["trade", "Trade %"],
              ] as const
            ).map(([key, label]) => (
              <TableHeader
                key={key}
                className="text-right"
                aria-sort={
                  sort.key === key ? (sort.descending ? "descending" : "ascending") : "none"
                }
              >
                <button
                  type="button"
                  className="inline-flex items-center gap-1 rounded focus-visible:outline-2 focus-visible:outline-blue-500"
                  onClick={() => setSort({ key, descending: sort.key !== key || !sort.descending })}
                >
                  {label}
                  {sort.key === key ? (
                    sort.descending ? (
                      <ArrowDown className="size-3" />
                    ) : (
                      <ArrowUp className="size-3" />
                    )
                  ) : null}
                </button>
              </TableHeader>
            ))}
            <TableHeader>
              <span className="sr-only">Insights</span>
            </TableHeader>
          </TableRow>
        </TableHead>
        <TableBody>
          {rows.map((item) => (
            <TableRow
              key={pairingKey(item)}
              href={`/account/pairings/${pairingKey(item)}?${query}`}
              title={`View ${pairingName(item.primaryCommanderId, item.secondaryCommanderId)} insights`}
            >
              <TableCell>
                <div className="flex items-center gap-3">
                  <PairingIdentity
                    primary={item.primaryCommanderId}
                    secondary={item.secondaryCommanderId}
                  />
                </div>
              </TableCell>
              <TableCell className="text-right tabular-nums text-zinc-400">
                {formatExact(item.count)}
              </TableCell>
              <TableCell
                className="text-right tabular-nums"
                title={formatExact(item.totals.killScore)}
              >
                {formatCompact(item.totals.killScore)}
              </TableCell>
              <TableCell
                className="text-right tabular-nums text-zinc-400"
                title={formatExact(item.totals.enemyKillScore)}
              >
                {formatCompact(item.totals.enemyKillScore)}
              </TableCell>
              <TableCell
                className={`text-right tabular-nums ${netKp(item.totals) >= 0 ? "text-green-400" : "text-red-400"}`}
                title={formatExact(netKp(item.totals))}
              >
                {netKp(item.totals) > 0 ? "+" : ""}
                {formatCompact(netKp(item.totals))}
              </TableCell>
              <TableCell className="text-right tabular-nums">{formatTrade(item.totals)}</TableCell>
              <TableCell>
                <ChevronRight className="size-4 text-zinc-500" aria-hidden="true" />
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </section>
  );
}
