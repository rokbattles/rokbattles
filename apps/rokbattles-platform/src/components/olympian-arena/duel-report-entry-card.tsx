"use client";

import { useTranslations } from "next-intl";
import { DuelParticipantCard } from "@/components/olympian-arena/duel-participant-card";
import { DuelResultsChart } from "@/components/olympian-arena/duel-results-chart";
import { Subheading } from "@/components/ui/heading";
import type { DuelReportEntry } from "@/hooks/use-olympian-arena-duel";
import { formatUtcDateTime } from "@/lib/datetime";
import type { DuelBattle2BattleResults } from "@/lib/types/duelbattle2";

type DuelEntryCardProps = {
  entry: DuelReportEntry;
};

export default function DuelReportEntryCard({ entry }: DuelEntryCardProps) {
  const t = useTranslations("duels");
  const { metadata, battle_results: battleResults, sender, opponent } = entry;

  const periodLabel = formatUtcDateTime(metadata.mail_time);
  const outcome = getOutcome(battleResults);

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
        <DuelResultsChart results={battleResults} />
      </section>

      <section className="grid gap-8 lg:grid-cols-2">
        <DuelParticipantCard participant={sender} isWinner={outcome?.winner === "sender"} />
        <DuelParticipantCard participant={opponent} isWinner={outcome?.winner === "opponent"} />
      </section>
    </section>
  );
}

function getOutcome(results: DuelBattle2BattleResults) {
  if (results.sender.win === true) {
    return { winner: "sender" as const };
  }

  if (results.opponent.win === true) {
    return { winner: "opponent" as const };
  }

  return null;
}
