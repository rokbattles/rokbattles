"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { ReportEntryCard } from "@/components/report/ReportEntryCard";
import { hasOverviewData, ReportOverviewCard } from "@/components/report/ReportOverviewCard";
import { Button } from "@/components/ui/Button";
import { Divider } from "@/components/ui/Divider";
import { Heading } from "@/components/ui/Heading";
import { Text } from "@/components/ui/Text";
import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import { useReport } from "@/hooks/useReport";
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

  const entries: ReportEntry[] = useMemo(() => data?.items ?? [], [data?.items]);
  const overviewSource = useMemo(() => {
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
  }, [entries]);

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

function ReportLoadingState() {
  return (
    <div className="space-y-6">
      {[0, 1].map((index) => (
        <div
          key={index}
          className="h-72 animate-pulse rounded-2xl border border-zinc-950/10 bg-zinc-100/80 dark:border-white/10 dark:bg-white/5"
        />
      ))}
    </div>
  );
}

function ReportErrorState({ message }: { message: string }) {
  return <Text>We could not load this report. {message}</Text>;
}

function ReportEmptyState() {
  return <Text>No battles were found for this hash.</Text>;
}
