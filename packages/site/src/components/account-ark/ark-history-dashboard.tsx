"use client";

import { cn } from "cn";
import dynamic from "next/dynamic";
import { useExtracted } from "next-intl";
import { type ReactElement, useState } from "react";
import { ArkMatchHistoryTable } from "@/components/account-ark/ark-match-history-table";
import {
  ArkMetricCards,
  ArkPagination,
  ArkSectionHeading,
  useArkLeagueLabels,
  useArkNumber,
} from "@/components/account-ark/ark-shared";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import { averageArkValue, getArkOutcome, getArkTeams } from "@/lib/ark/analytics";
import type { ArkMatchHistoryResult } from "@/lib/types/ark";

const ArkHistoryCharts = dynamic(
  () =>
    import("@/components/account-ark/ark-history-charts").then((module) => module.ArkHistoryCharts),
  {
    loading: () => <div className="h-80 rounded-md bg-zinc-900" aria-hidden="true" />,
  }
);

type ArkHistoryDashboardProps = { data: ArkMatchHistoryResult };

const LEAGUES = ["all", "golden", "silver", "osiris"] as const;

export function ArkHistoryDashboard({ data }: ArkHistoryDashboardProps): ReactElement {
  const t = useExtracted();
  const labels = useArkLeagueLabels();
  const number = useArkNumber();

  const [league, setLeague] = useState<(typeof LEAGUES)[number]>("all");
  const [page, setPage] = useState(1);

  const rows =
    league === "all" ? data.items : data.items.filter((match) => match.league === league);

  const wins = rows.filter((match) => getArkOutcome(match) === "win").length;
  const losses = rows.filter((match) => getArkOutcome(match) === "loss").length;
  const draws = rows.filter((match) => getArkOutcome(match) === "draw").length;
  const decided = wins + losses + draws;

  const pages = Math.max(1, Math.ceil(rows.length / 10));
  const currentPage = Math.min(page, pages);

  return (
    <div className="mt-4 space-y-8">
      <Text>{t("Your matches, your team, and the moments that made the difference.")}</Text>
      <nav
        aria-label={t("Ark leagues")}
        className="flex gap-6 overflow-x-auto border-b border-zinc-800"
      >
        {LEAGUES.map((key) => (
          <button
            key={key}
            type="button"
            aria-pressed={league === key}
            onClick={() => {
              setLeague(key);
              setPage(1);
            }}
            className={cn(
              "shrink-0 border-b-2 pb-3 text-sm font-medium",
              "focus-visible:outline-2 focus-visible:outline-blue-500",
              league === key
                ? "border-white text-white"
                : "border-transparent text-zinc-400 hover:text-white"
            )}
          >
            {labels[key]}
          </button>
        ))}
      </nav>
      <section className="space-y-4">
        <ArkSectionHeading
          title={t("Match overview")}
          description={t("Results and averages for the selected league.")}
        />
        <ArkMetricCards
          items={[
            {
              label: t("Matches"),
              value: number(rows.length),
              detail: t("{wins} wins · {losses} losses · {draws} draws", {
                wins: number(wins),
                losses: number(losses),
                draws: number(draws),
              }),
            },
            {
              label: t("Win rate"),
              value: decided ? `${number((wins / decided) * 100)}%` : "—",
            },
            {
              label: t("Average alliance score"),
              value: number(averageArkValue(rows.map((match) => getArkTeams(match).own?.score))),
            },
            {
              label: t("Average individual score"),
              value: number(averageArkValue(rows.map((match) => match.personalScore))),
            },
          ]}
        />
      </section>
      {rows.length ? (
        <>
          <ArkHistoryCharts rows={rows} />
          <section className="space-y-4">
            <ArkSectionHeading
              title={t("Match history")}
              description={t("Open a match to explore scores, pairings, and battle reports.")}
            />
            <ArkMatchHistoryTable rows={rows.slice((currentPage - 1) * 10, currentPage * 10)} />
            <ArkPagination page={currentPage} pages={pages} onChange={setPage} />
          </section>
        </>
      ) : (
        <div className="rounded-md border border-dashed px-6 py-10 text-center border-zinc-700">
          <Subheading>{t("No matches in this league.")}</Subheading>
          <Text className="mt-2">
            {t("Try another league or upload Ark reports from the desktop app.")}
          </Text>
        </div>
      )}
    </div>
  );
}
