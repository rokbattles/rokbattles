"use client";

import { usePathname } from "next/navigation";
import { parseAsStringLiteral, useQueryStates } from "nuqs";
import { Suspense } from "react";
import { CombatLabRankingsLoading } from "@/components/combat-lab/combat-lab-rankings-loading";
import {
  CombatLabRankingsFrame,
  CombatLabRankingsResults,
} from "@/components/combat-lab/combat-lab-rankings-results";
import { useClientReady } from "@/hooks/use-client-ready";
import {
  type CombatLabRankingDirection,
  type CombatLabRankingSort,
  combatLabRankingDirections,
  combatLabRankingSorts,
} from "@/lib/combat-lab/rankings-api";
import { combatLabSeason } from "@/lib/combat-lab/season";

export function CombatLabRankingsContent() {
  const season = combatLabSeason(usePathname());
  const clientReady = useClientReady();
  const [sorting, setSorting] = useQueryStates(
    {
      sort: parseAsStringLiteral(combatLabRankingSorts).withDefault("overall"),
      direction: parseAsStringLiteral(combatLabRankingDirections).withDefault("desc"),
    },
    {
      clearOnDefault: false,
      history: "push",
    }
  );

  const handleSort = (sort: CombatLabRankingSort) => {
    const direction: CombatLabRankingDirection =
      sorting.sort === sort && sorting.direction === "desc" ? "asc" : "desc";
    setSorting({ sort, direction });
  };

  if (!clientReady) {
    return (
      <CombatLabRankingsFrame>
        <CombatLabRankingsLoading />
      </CombatLabRankingsFrame>
    );
  }

  return (
    <Suspense
      key={`${season}:${sorting.sort}:${sorting.direction}`}
      fallback={
        <CombatLabRankingsFrame>
          <CombatLabRankingsLoading />
        </CombatLabRankingsFrame>
      }
    >
      <CombatLabRankingsResults
        season={season}
        sort={sorting.sort}
        direction={sorting.direction}
        onSort={handleSort}
      />
    </Suspense>
  );
}
