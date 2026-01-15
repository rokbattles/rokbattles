"use client";

import { useTranslations } from "next-intl";
import { DuelParticipantCard } from "@/components/olympian-arena/duel-participant-card";
import { DuelResultsChart } from "@/components/olympian-arena/duel-results-chart";
import { Subheading } from "@/components/ui/Heading";
import type { DuelReportEntry } from "@/hooks/use-olympian-arena-duel";
import { formatUtcDateTime } from "@/lib/datetime";
import type { DuelResults } from "@/lib/types/duel-report";

type DuelEntryCardProps = {
  entry: DuelReportEntry;
};

export default function DuelReportEntryCard({ entry }: DuelEntryCardProps) {
  const t = useTranslations("duels");
  const payload = entry.report;
  const { metadata, results, sender, opponent } = payload;

  const periodLabel = formatUtcDateTime(metadata.email_time);
  const outcome = getOutcome(results);

  return (
    <section className="space-y-6">
      <header className="flex flex-wrap items-center gap-3">
        <Subheading level={3} className="text-lg">
          {periodLabel}
        </Subheading>
      </header>

      <section className="space-y-4">
        <Subheading level={3} className="text-base">
          {t("summary")}
        </Subheading>
        <DuelResultsChart results={results} />
      </section>

      <section className="grid gap-8 lg:grid-cols-2">
        <DuelParticipantCard participant={sender} isWinner={outcome?.winner === "sender"} />
        <DuelParticipantCard participant={opponent} isWinner={outcome?.winner === "opponent"} />
      </section>
    </section>
  );
}

function getOutcome(results: DuelResults) {
  if (results.win === true) {
    return { winner: "sender" as const };
  }

  if (results.opponent_win === true) {
    return { winner: "opponent" as const };
  }

  return null;
}
