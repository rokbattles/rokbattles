"use client";

import { parseAsStringLiteral, useQueryStates } from "nuqs";
import { Suspense } from "react";
import { CombatLabRankingsLoading } from "@/components/combat-lab/combat-lab-rankings-loading";
import { CombatLabRankingsResults } from "@/components/combat-lab/combat-lab-rankings-results";
import { useClientReady } from "@/hooks/use-client-ready";
import {
  type CombatLabRankingDirection,
  type CombatLabRankingSort,
  combatLabRankingDirections,
  combatLabRankingSorts,
} from "@/lib/combat-lab/rankings-api";

export function CombatLabRankingsContent() {
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
    return <CombatLabRankingsLoading />;
  }

  return (
    <Suspense key={`${sorting.sort}:${sorting.direction}`} fallback={<CombatLabRankingsLoading />}>
      <CombatLabRankingsResults
        sort={sorting.sort}
        direction={sorting.direction}
        onSort={handleSort}
      />
    </Suspense>
  );
}
