"use client";

import { useLocale } from "next-intl";
import { useLayoutEffect, useRef } from "react";
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
import { type CombatLabSeason, combatLabPath } from "@/lib/combat-lab/season";
import { getCommanderName } from "@/lib/commander";

const overallColumn = { key: "overall", label: "DRASTC" } as const;
const breakdownColumns: Array<{
  key: Exclude<CombatLabRankingSort, "overall">;
  label: string;
  title: string;
}> = [
  { key: "damage", label: "D", title: "Damage" },
  { key: "rage", label: "R", title: "Rage" },
  { key: "assist", label: "A", title: "Assist" },
  { key: "sustainability", label: "S", title: "Sustainability" },
  { key: "trade", label: "T", title: "Trade" },
  { key: "consistency", label: "C", title: "Consistency" },
];

type CombatLabRankingsTableProps = {
  season: CombatLabSeason;
  items: CombatLabRanking[];
  sort: CombatLabRankingSort;
  direction: CombatLabRankingDirection;
  onSort: (sort: CombatLabRankingSort) => void;
};

export function CombatLabRankingsTable({
  season,
  items,
  sort,
  direction,
  onSort,
}: CombatLabRankingsTableProps) {
  const locale = useLocale();
  const headRef = useRef<HTMLTableSectionElement>(null);

  useLayoutEffect(() => {
    const head = headRef.current;
    const table = head?.closest("table");
    if (!head || !table) return;

    let frame = 0;
    const updatePosition = () => {
      const { top, height } = table.getBoundingClientRect();
      const offset = Math.max(0, Math.min(-top, height - head.offsetHeight));
      head.style.transform = `translateY(${offset}px)`;
    };
    const scheduleUpdate = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(updatePosition);
    };

    window.addEventListener("scroll", scheduleUpdate, { capture: true, passive: true });
    window.addEventListener("resize", scheduleUpdate);
    const observer = new ResizeObserver(scheduleUpdate);
    observer.observe(table);
    updatePosition();

    return () => {
      window.removeEventListener("scroll", scheduleUpdate, true);
      window.removeEventListener("resize", scheduleUpdate);
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, []);

  return (
    <Table dense>
      <TableHead ref={headRef} className="relative z-10 bg-white dark:bg-zinc-900">
        <TableRow>
          <TableHeader className="w-12 min-w-12 text-right">#</TableHeader>
          <TableHeader>Pairing</TableHeader>
          <CombatLabRankingsSortableHeader
            label={overallColumn.label}
            column={overallColumn.key}
            sort={sort}
            direction={direction}
            onSort={onSort}
            className="w-0 px-2"
          />
          <TableHeader className="w-0 px-2 text-right">
            <span className="inline-flex items-center gap-1">
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
              className="w-0 px-2"
              title={column.title}
            />
          ))}
        </TableRow>
      </TableHead>
      <TableBody>
        {items.map((item, index) => {
          const primaryName = getCommanderName(item.primaryCommanderId, locale) ?? "Unknown";
          const secondaryName = getCommanderName(item.secondaryCommanderId, locale) ?? "Unknown";
          const href = `${combatLabPath(season)}?primary=${item.primaryCommanderId}&secondary=${item.secondaryCommanderId}`;

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
                <div className="flex flex-col">
                  <span className="inline-flex items-center gap-2">
                    <CommanderIcon
                      id={item.primaryCommanderId}
                      alt={`${primaryName} icon`}
                      className="size-8 rounded-full"
                    />
                    <span>{primaryName}</span>
                  </span>
                  <span className="inline-flex items-center gap-2 text-zinc-600 dark:text-zinc-400">
                    <CommanderIcon
                      id={item.secondaryCommanderId}
                      alt={`${secondaryName} icon`}
                      className="size-8 rounded-full"
                    />
                    <span>{secondaryName}</span>
                  </span>
                </div>
              </TableCell>
              <CombatLabRankingsScoreCell className="w-0 px-2" score={item.drastc.overall} />
              <TableCell className="w-0 px-2 text-right text-zinc-600 tabular-nums dark:text-zinc-400">
                {scoreFormatter.format(
                  Math.min(99.99, clampScore(item.drastc.confidence.score) * 10)
                )}
                %
              </TableCell>
              <CombatLabRankingsScoreCell
                className="w-0 px-2"
                score={item.drastc.breakdown.damage}
              />
              <CombatLabRankingsScoreCell className="w-0 px-2" score={item.drastc.breakdown.rage} />
              <CombatLabRankingsScoreCell
                className="w-0 px-2"
                score={item.drastc.breakdown.assist}
              />
              <CombatLabRankingsScoreCell
                className="w-0 px-2"
                score={item.drastc.breakdown.sustainability}
              />
              <CombatLabRankingsScoreCell
                className="w-0 px-2"
                score={item.drastc.breakdown.trade}
              />
              <CombatLabRankingsScoreCell
                className="w-0 px-2"
                score={item.drastc.breakdown.consistency}
              />
            </TableRow>
          );
        })}
      </TableBody>
    </Table>
  );
}
