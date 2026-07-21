"use client";

import { ArrowDownIcon, ArrowsUpDownIcon, ArrowUpIcon } from "@heroicons/react/16/solid";
import { TableHeader } from "@/components/ui/table";
import type {
  CombatLabRankingDirection,
  CombatLabRankingSort,
} from "@/lib/combat-lab/rankings-api";

type CombatLabRankingsSortableHeaderProps = {
  label: string;
  column: CombatLabRankingSort;
  sort: CombatLabRankingSort;
  direction: CombatLabRankingDirection;
  onSort: (sort: CombatLabRankingSort) => void;
};

export function CombatLabRankingsSortableHeader({
  label,
  column,
  sort,
  direction,
  onSort,
}: CombatLabRankingsSortableHeaderProps) {
  const active = sort === column;
  const Icon = getSortIcon(active, direction);
  const ariaSort = getAriaSort(active, direction);
  const nextDirection = active && direction === "desc" ? "ascending" : "descending";

  return (
    <TableHeader aria-sort={ariaSort} className="text-right">
      <button
        type="button"
        className="inline-flex items-center gap-1.5 hover:text-zinc-950 dark:hover:text-white"
        aria-label={`Sort by ${label} ${nextDirection}`}
        onClick={() => onSort(column)}
      >
        <span>{label}</span>
        <Icon aria-hidden="true" className="size-4" />
      </button>
    </TableHeader>
  );
}

function getSortIcon(active: boolean, direction: CombatLabRankingDirection) {
  if (!active) {
    return ArrowsUpDownIcon;
  }

  return direction === "asc" ? ArrowUpIcon : ArrowDownIcon;
}

function getAriaSort(
  active: boolean,
  direction: CombatLabRankingDirection
): "ascending" | "descending" | "none" {
  if (!active) {
    return "none";
  }

  return direction === "asc" ? "ascending" : "descending";
}
