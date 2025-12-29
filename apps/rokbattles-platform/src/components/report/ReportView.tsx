"use client";

import { useEffect, useRef, useState } from "react";
import { ReportEmptyState } from "@/components/report/ReportEmptyState";
import { ReportEntryCard } from "@/components/report/ReportEntryCard";
import { ReportErrorState } from "@/components/report/ReportErrorState";
import { ReportLoadingState } from "@/components/report/ReportLoadingState";
import { ReportOverviewCard } from "@/components/report/ReportOverviewCard";
import { Button } from "@/components/ui/Button";
import { Divider } from "@/components/ui/Divider";
import { Heading } from "@/components/ui/Heading";
import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import { useReport } from "@/hooks/useReport";
import { hasOverviewData } from "@/lib/report/overviewMetrics";
import type { RawOverview, RawReportPayload } from "@/lib/types/rawReport";
import type { ReportEntry } from "@/lib/types/report";

type ReportViewProps = {
  hash: string;
};

export function ReportView({ hash }: ReportViewProps) {
  const normalizedHash = hash?.trim() ?? "";

  const { data, loading, error } = useReport(normalizedHash.length > 0 ? normalizedHash : null);
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

  const entries: ReportEntry[] = data?.items ?? [];
  const overviewSource = findOverviewSource(entries);

  function handleShare() {
    copy(`https://platform.rokbattles.com/report/${normalizedHash}`)
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
        <Button className="-my-0.5" disabled={isCopied} onClick={handleShare}>
          {isCopied ? "Copied" : "Share"}
        </Button>
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
          {overviewSource && hasOverviewData(overviewSource.overview) ? (
            <>
              <ReportOverviewCard
                overview={overviewSource.overview}
                selfParticipant={overviewSource.self}
                enemyParticipant={overviewSource.enemy}
              />
              <Divider />
            </>
          ) : null}
          <div className="space-y-12">
            {entries.map((entry, index) => (
              <div key={entry.hash}>
                <ReportEntryCard entry={entry} />
                {index < entries.length - 1 ? <Divider className="my-8" /> : null}
              </div>
            ))}
          </div>
        </div>
      )}
    </section>
  );
}

function findOverviewSource(entries: ReportEntry[]) {
  for (const entry of entries) {
    const payload = (entry.report ?? {}) as RawReportPayload;
    if (payload?.overview && typeof payload.overview === "object") {
      return {
        overview: payload.overview as RawOverview,
        self: payload.self,
        enemy: payload.enemy,
      };
    }
  }
  return null;
}
