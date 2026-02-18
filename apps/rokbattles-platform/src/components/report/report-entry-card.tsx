"use client";

import { useExtracted } from "next-intl";
import { ReportBattleResultsChart } from "@/components/report/report-battle-results-chart";
import { ReportParticipantCard } from "@/components/report/report-participant-card";
import { Subheading } from "@/components/ui/heading";
import type { RawReportPayload } from "@/lib/types/raw-report";
import type { ReportEntry } from "@/lib/types/report";

type ReportEntryCardProps = {
  entry: ReportEntry;
};

export function ReportEntryCard({ entry }: ReportEntryCardProps) {
  const t = useExtracted();
  const payload = (entry.report ?? {}) as RawReportPayload;

  const metadata = payload?.metadata;
  const battleResults = payload?.battle_results;
  const selfParticipant = payload?.self;
  const enemyParticipant = payload?.enemy;
  const omitArtifacts = metadata?.email_role === "dungeon" || metadata?.is_kvk === 0;

  const start = metadata?.start_date ?? entry.startDate;
  const end = metadata?.end_date;
  const periodLabel = formatPeriod(start, end, {
    unknownPeriodLabel: t("Unknown period"),
    utcPrefix: t("UTC "),
  });

  return (
    <section className="space-y-6">
      <header className="flex flex-wrap items-center gap-3">
        <Subheading level={3} className="text-lg">
          {periodLabel}
        </Subheading>
      </header>

      {battleResults ? (
        <section className="space-y-4">
          <Subheading level={3} className="text-base">
            {t("Battle summary")}
          </Subheading>
          <ReportBattleResultsChart results={battleResults} />
        </section>
      ) : null}
      <section className="grid gap-8 lg:grid-cols-2">
        <ReportParticipantCard participant={selfParticipant} showArtifacts={!omitArtifacts} />
        <ReportParticipantCard participant={enemyParticipant} showArtifacts={!omitArtifacts} />
      </section>
    </section>
  );
}

function formatPeriod(
  start: number | null | undefined,
  end: number | null | undefined,
  labels: { unknownPeriodLabel: string; utcPrefix: string }
): string {
  const startMs = toMillis(start);
  if (startMs == null) {
    return labels.unknownPeriodLabel;
  }

  const startDate = new Date(startMs);
  const startLabel = formatUtc(startDate, labels.utcPrefix);

  const endMs = toMillis(end);
  if (endMs == null) {
    return startLabel;
  }

  const endDate = new Date(endMs);
  const sameDay =
    startDate.getUTCFullYear() === endDate.getUTCFullYear() &&
    startDate.getUTCMonth() === endDate.getUTCMonth() &&
    startDate.getUTCDate() === endDate.getUTCDate();

  const endLabel = sameDay
    ? formatUtc(endDate, labels.utcPrefix, { includeDate: false, includePrefix: false })
    : formatUtc(endDate, labels.utcPrefix, { includePrefix: false });

  return `${startLabel} - ${endLabel}`;
}

function formatUtc(
  date: Date,
  utcPrefix: string,
  options: { includeDate?: boolean; includePrefix?: boolean } = { includeDate: true }
) {
  const includeDate = options.includeDate ?? true;
  const includePrefix = options.includePrefix ?? true;
  const month = String(date.getUTCMonth() + 1).padStart(2, "0");
  const day = String(date.getUTCDate()).padStart(2, "0");
  const hours = String(date.getUTCHours()).padStart(2, "0");
  const minutes = String(date.getUTCMinutes()).padStart(2, "0");

  const prefix = includePrefix ? utcPrefix : "";
  return includeDate
    ? `${prefix}${month}/${day} ${hours}:${minutes}`
    : `${prefix}${hours}:${minutes}`;
}

function toMillis(value?: number | null): number | null {
  if (value == null || !Number.isFinite(value)) {
    return null;
  }

  const numeric = Number(value);
  const absValue = Math.abs(numeric);
  const millis = absValue < 1_000_000_000_000 ? numeric * 1000 : numeric;
  return Number.isFinite(millis) ? millis : null;
}
