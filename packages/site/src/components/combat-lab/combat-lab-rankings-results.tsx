"use client";

import { useExtracted, useLocale } from "next-intl";
import { use } from "react";
import { CombatLabHeader } from "@/components/combat-lab/combat-lab-header";
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
  const locale = useLocale();
  const result = use(loadCombatLabRankingsResult({ sort, direction }));

  if (result.status === "error") {
    return (
      <CombatLabRankingsFrame>
        <CombatLabMessage
          title="Combat Lab rankings are unavailable"
          message="The rankings could not be loaded right now. Please try again shortly."
        />
      </CombatLabRankingsFrame>
    );
  }

  return (
    <CombatLabRankingsFrame
      lastUpdated={
        result.refreshedAt
          ? t("Last updated: {date}", {
              date: formatRefreshedAt(result.refreshedAt, locale),
            })
          : undefined
      }
    >
      <CombatLabRankingsTable
        items={result.items}
        sort={sort}
        direction={direction}
        onSort={onSort}
      />
    </CombatLabRankingsFrame>
  );
}

export function CombatLabRankingsFrame({
  children,
  lastUpdated,
}: {
  children: React.ReactNode;
  lastUpdated?: string;
}) {
  return (
    <div className="min-h-dvh text-zinc-950 dark:text-white">
      <CombatLabHeader active="rankings">
        {lastUpdated ? <Text className="mt-4 !text-sm/6 !text-zinc-400">{lastUpdated}</Text> : null}
      </CombatLabHeader>
      <div className="mx-auto max-w-7xl px-4 pt-7 sm:px-6 sm:pt-10 lg:px-8">{children}</div>
    </div>
  );
}
