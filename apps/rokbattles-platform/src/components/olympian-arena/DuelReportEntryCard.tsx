"use client";

import { useTranslations } from "next-intl";
import { DuelParticipantCard } from "@/components/olympian-arena/DuelParticipantCard";
import { DuelResultsChart } from "@/components/olympian-arena/DuelResultsChart";
import { Subheading } from "@/components/ui/Heading";
import type { DuelReportEntry } from "@/hooks/useOlympianArenaDuel";
import { formatUtcDateTime } from "@/lib/datetime";
import type { DuelReportPayload, DuelResults } from "@/lib/types/duelReport";

type DuelEntryCardProps = {
  entry: DuelReportEntry;
};

export default function DuelReportEntryCard({ entry }: DuelEntryCardProps) {
  const t = useTranslations("duels");
  const payload = (entry.report ?? {}) as DuelReportPayload;
  const metadata = payload.metadata;
  const results = payload.results;
  const sender = payload.sender;
  const opponent = payload.opponent;

  const periodLabel = formatUtcDateTime(metadata?.email_time);
  const outcome = getOutcome(results);

  return (
    <section className="space-y-6">
      <header className="flex flex-wrap items-center gap-3">
        <Subheading level={3} className="text-lg">
          {periodLabel}
        </Subheading>
      </header>

      {results ? (
        <section className="space-y-4">
          <Subheading level={3} className="text-base">
            {t("summary")}
          </Subheading>
          <DuelResultsChart results={results} />
        </section>
      ) : null}

      <section className="grid gap-8 lg:grid-cols-2">
        <DuelParticipantCard participant={sender} isWinner={outcome?.winner === "sender"} />
        <DuelParticipantCard participant={opponent} isWinner={outcome?.winner === "opponent"} />
      </section>
    </section>
  );
}

function getOutcome(results?: DuelResults) {
  if (!results) {
    return null;
  }

  if (results.win === true) {
    return { winner: "sender" as const };
  }

  if (results.opponent_win === true) {
    return { winner: "opponent" as const };
  }

  return null;
}
