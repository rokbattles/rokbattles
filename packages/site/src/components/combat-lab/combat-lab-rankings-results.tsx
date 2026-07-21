"use client";

import { useExtracted } from "next-intl";
import { use } from "react";
import { CombatLabMessage } from "@/components/combat-lab/combat-lab-message";
import { CombatLabRankingsTable } from "@/components/combat-lab/combat-lab-rankings-table";
import { Text } from "@/components/ui/text";
import { formatRefreshedAt } from "@/lib/combat-lab/format";
import {
  type CombatLabRankingDirection,
  type CombatLabRankingSort,
  loadCombatLabRankingsResult,
} from "@/lib/combat-lab/rankings-api";

type CombatLabRankingsResultsProps = {
  sort: CombatLabRankingSort;
  direction: CombatLabRankingDirection;
  onSort: (sort: CombatLabRankingSort) => void;
};

export function CombatLabRankingsResults({
  sort,
  direction,
  onSort,
}: CombatLabRankingsResultsProps) {
  const t = useExtracted();
  const result = use(loadCombatLabRankingsResult({ sort, direction }));

  if (result.status === "error") {
    return (
      <CombatLabMessage
        title="Combat Lab rankings are unavailable"
        message="The rankings could not be loaded right now. Please try again shortly."
      />
    );
  }

  return (
    <div className="space-y-4">
      {result.refreshedAt ? (
        <Text>{t("Last updated: {date}", { date: formatRefreshedAt(result.refreshedAt) })}</Text>
      ) : null}
      <CombatLabRankingsTable
        items={result.items}
        sort={sort}
        direction={direction}
        onSort={onSort}
      />
    </div>
  );
}
