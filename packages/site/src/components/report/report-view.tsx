"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { ReportEmptyState } from "@/components/report/report-empty-state";
import { ReportEntryList } from "@/components/report/report-entry-list";
import { ReportErrorState } from "@/components/report/report-error-state";
import { ReportLoadingState } from "@/components/report/report-loading-state";
import { ReportOverviewCard } from "@/components/report/report-overview-card";
import { Button } from "@/components/ui/button";
import { Divider } from "@/components/ui/divider";
import { Heading } from "@/components/ui/heading";
import { useCopyToClipboard } from "@/hooks/use-copy-to-clipboard";
import { useReport } from "@/hooks/use-report";
import { adaptBattleMailToReport } from "@/lib/report/battle-mail-adapter";
import { hasOverviewData } from "@/lib/report/overview-metrics";
import type { ReportEntry } from "@/lib/types/report";

type ReportViewProps = {
  id: string;
};

export function ReportView({ id }: ReportViewProps) {
  const normalizedId = id?.trim() ?? "";

  const { data, loading, error } = useReport(normalizedId.length > 0 ? normalizedId : null);

  const [copiedText, copy] = useCopyToClipboard();
  const [isCopied, setIsCopied] = useState(false);
  const resetTimerRef = useRef<number>(null);

  useEffect(() => {
    return () => {
      if (resetTimerRef.current) {
        clearTimeout(resetTimerRef.current);
      }
    };
  }, []);

  const mappedReport = useMemo(
    () => (data?.mail ? adaptBattleMailToReport(data.mail) : null),
    [data?.mail]
  );

  const entries: ReportEntry[] = mappedReport?.entries ?? [];
  const overview = mappedReport?.overview ?? null;
  const selfParticipant = mappedReport?.selfParticipant;
  const enemyParticipant = mappedReport?.enemyParticipant;

  function handleShare() {
    copy(`https://rokbattles.com/report/${encodeURIComponent(normalizedId)}`)
      .then(() => console.log("Battle report copied to clipboard", copiedText))
      .then(() => {
        setIsCopied(true);

        if (resetTimerRef.current) {
          clearTimeout(resetTimerRef.current);
        }

        resetTimerRef.current = window.setTimeout(() => setIsCopied(false), 2000);
      })
      .catch((err) => console.error("Failed to copy battle report to clipboard", err));
  }

  return (
    <section className="space-y-8">
      <div className="flex items-end justify-between gap-4">
        <Heading>Report</Heading>
        <div className="flex items-center gap-2">
          <Button className="-my-0.5" disabled={isCopied} onClick={handleShare}>
            {isCopied ? "Copied" : "Share"}
          </Button>
        </div>
      </div>

      <Divider />

      {loading ? (
        <ReportLoadingState />
      ) : error ? (
        <ReportErrorState message={error} />
      ) : entries.length === 0 ? (
        <ReportEmptyState />
      ) : (
        <div className="space-y-12">
          {overview && hasOverviewData(overview) ? (
            <>
              <ReportOverviewCard
                overview={overview}
                selfParticipant={selfParticipant}
                enemyParticipant={enemyParticipant}
              />
              <Divider />
            </>
          ) : null}

          <ReportEntryList key={normalizedId} entries={entries} />
        </div>
      )}
    </section>
  );
}
