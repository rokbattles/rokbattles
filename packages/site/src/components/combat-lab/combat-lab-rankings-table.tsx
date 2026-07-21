"use client";

import { useLocale } from "next-intl";
import { CombatLabRankingsScoreCell } from "@/components/combat-lab/combat-lab-rankings-score-cell";
import { CombatLabRankingsSortableHeader } from "@/components/combat-lab/combat-lab-rankings-sortable-header";
import { CommanderIcon } from "@/components/commander-icon";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { clampScore, scoreFormatter } from "@/lib/combat-lab/format";
import type {
  CombatLabRanking,
  CombatLabRankingDirection,
  CombatLabRankingSort,
} from "@/lib/combat-lab/rankings-api";
import { getCommanderName } from "@/lib/commander";

const overallColumn = { key: "overall", label: "DRASTC" } as const;
const breakdownColumns: Array<{ key: Exclude<CombatLabRankingSort, "overall">; label: string }> = [
  { key: "damage", label: "Damage" },
  { key: "rage", label: "Rage" },
  { key: "assist", label: "Assist" },
  { key: "sustainability", label: "Sustainability" },
  { key: "trade", label: "Trade" },
  { key: "consistency", label: "Consistency" },
];

type CombatLabRankingsTableProps = {
  items: CombatLabRanking[];
  sort: CombatLabRankingSort;
  direction: CombatLabRankingDirection;
  onSort: (sort: CombatLabRankingSort) => void;
};

export function CombatLabRankingsTable({
  items,
  sort,
  direction,
  onSort,
}: CombatLabRankingsTableProps) {
  const locale = useLocale();

  return (
    <Table dense>
      <TableHead>
        <TableRow>
          <TableHeader className="w-12 min-w-12 text-right">#</TableHeader>
          <TableHeader>Pairing</TableHeader>
          <CombatLabRankingsSortableHeader
            label={overallColumn.label}
            column={overallColumn.key}
            sort={sort}
            direction={direction}
            onSort={onSort}
          />
          <TableHeader className="text-right">
            <span className="inline-flex items-center gap-2">
              Confidence
              <Badge>Beta</Badge>
            </span>
          </TableHeader>
          {breakdownColumns.map((column) => (
            <CombatLabRankingsSortableHeader
              key={column.key}
              label={column.label}
              column={column.key}
              sort={sort}
              direction={direction}
              onSort={onSort}
            />
          ))}
        </TableRow>
      </TableHead>
      <TableBody>
        {items.map((item, index) => {
          const primaryName = getCommanderName(item.primaryCommanderId, locale) ?? "Unknown";
          const secondaryName = getCommanderName(item.secondaryCommanderId, locale) ?? "Unknown";
          const href = `/combat-lab?primary=${item.primaryCommanderId}&secondary=${item.secondaryCommanderId}`;

          return (
            <TableRow
              key={`${item.primaryCommanderId}:${item.secondaryCommanderId}`}
              href={href}
              title={`Explore ${primaryName} and ${secondaryName}`}
            >
              <TableCell className="w-12 min-w-12 text-right text-zinc-600 tabular-nums dark:text-zinc-400">
                {index + 1}
              </TableCell>
              <TableCell>
                <div className="pointer-events-none flex -space-x-2">
                  <CommanderIcon
                    id={item.primaryCommanderId}
                    alt={`${primaryName} icon`}
                    className="size-10"
                  />
                  <CommanderIcon
                    id={item.secondaryCommanderId}
                    alt={`${secondaryName} icon`}
                    className="size-10"
                  />
                </div>
              </TableCell>
              <CombatLabRankingsScoreCell score={item.drastc.overall} />
              <TableCell className="text-right text-zinc-600 tabular-nums dark:text-zinc-400">
                {scoreFormatter.format(
                  Math.min(99.99, clampScore(item.drastc.confidence.score) * 10)
                )}
                %
              </TableCell>
              <CombatLabRankingsScoreCell score={item.drastc.breakdown.damage} />
              <CombatLabRankingsScoreCell score={item.drastc.breakdown.rage} />
              <CombatLabRankingsScoreCell score={item.drastc.breakdown.assist} />
              <CombatLabRankingsScoreCell score={item.drastc.breakdown.sustainability} />
              <CombatLabRankingsScoreCell score={item.drastc.breakdown.trade} />
              <CombatLabRankingsScoreCell score={item.drastc.breakdown.consistency} />
            </TableRow>
          );
        })}
      </TableBody>
    </Table>
  );
}
