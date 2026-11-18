"use client";

import { ArrowDown, ArrowUp } from "lucide-react";
import { useState } from "react";
import { PairingIdentity, PairingsState } from "@/components/my-pairings/pairings-shared";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { PairingMatchup, PairingStats } from "@/lib/pairings";
import { formatCompact, formatExact } from "@/lib/pairings/analytics";
import { formatDetailTrade } from "@/lib/pairings/detail";

type SortKey = "battles" | "kpGain" | "kpLoss" | "netKp" | "tradePercent";

const COLUMNS = [
  ["battles", "Battles"],
  ["kpGain", "KP Gain"],
  ["kpLoss", "KP Loss"],
  ["netKp", "Net KP"],
  ["tradePercent", "Trade %"],
] as const;

function sortValue(stats: PairingStats, key: SortKey) {
  return key === "tradePercent"
    ? (stats.tradePercent ?? (stats.kpGain > 0 ? Number.POSITIVE_INFINITY : -1))
    : stats[key];
}

export function PairingMatchups({
  items,
  loading,
  error,
  retry,
}: {
  items: PairingMatchup[];
  loading: boolean;
  error: string | null;
  retry: () => void;
}) {
  if (loading) return <PairingsState loading />;
  if (error) return <PairingsState error={error} retry={retry} />;
  return <MatchupsTable items={items} />;
}

function MatchupsTable({ items }: { items: PairingMatchup[] }) {
  const [limit, setLimit] = useState(20);
  const [sort, setSort] = useState<{ key: SortKey; descending: boolean }>({
    key: "battles",
    descending: true,
  });

  const rows = items.toSorted((a, b) => {
    const left = sortValue(a.stats, sort.key);
    const right = sortValue(b.stats, sort.key);
    return left === right
      ? b.stats.battles - a.stats.battles
      : (left > right ? 1 : -1) * (sort.descending ? -1 : 1);
  });

  if (!rows.length)
    return <PairingsState>No matchups with 100+ battles in this range.</PairingsState>;

  return (
    <div className="space-y-4">
      <Table dense className="[&>div]:w-full [&_table]:w-full [&_table]:min-w-[48rem]">
        <caption className="sr-only">
          Opponent pairings with at least 100 battles for the selected loadout
        </caption>
        <TableHead>
          <TableRow>
            <TableHeader>Pairing</TableHeader>
            {COLUMNS.map(([key, label]) => (
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
                      <ArrowDown className="size-3" aria-hidden="true" />
                    ) : (
                      <ArrowUp className="size-3" aria-hidden="true" />
                    )
                  ) : null}
                </button>
              </TableHeader>
            ))}
          </TableRow>
        </TableHead>
        <TableBody>
          {rows.slice(0, limit).map((item) => (
            <TableRow key={`${item.primaryCommanderId}-${item.secondaryCommanderId}`}>
              <TableCell>
                <PairingIdentity
                  primary={item.primaryCommanderId}
                  secondary={item.secondaryCommanderId}
                />
              </TableCell>
              <TableCell className="text-right tabular-nums text-zinc-400">
                {formatExact(item.stats.battles)}
              </TableCell>
              <TableCell className="text-right tabular-nums" title={formatExact(item.stats.kpGain)}>
                {formatCompact(item.stats.kpGain)}
              </TableCell>
              <TableCell
                className="text-right tabular-nums text-zinc-400"
                title={formatExact(item.stats.kpLoss)}
              >
                {formatCompact(item.stats.kpLoss)}
              </TableCell>
              <TableCell
                className={`text-right tabular-nums ${item.stats.netKp >= 0 ? "text-green-400" : "text-red-400"}`}
                title={formatExact(item.stats.netKp)}
              >
                {item.stats.netKp > 0 ? "+" : ""}
                {formatCompact(item.stats.netKp)}
              </TableCell>
              <TableCell className="text-right tabular-nums">
                {formatDetailTrade(item.stats)}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      {limit < rows.length ? (
        <div className="flex justify-end">
          <Button plain onClick={() => setLimit((value) => value + 20)}>
            Show more
          </Button>
        </div>
      ) : null}
    </div>
  );
}
