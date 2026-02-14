"use client";

import { useTranslations } from "next-intl";
import { useEffect, useRef, useState } from "react";
import { ReportEmptyState } from "@/components/report/report-empty-state";
import { ReportEntryCard } from "@/components/report/report-entry-card";
import { ReportErrorState } from "@/components/report/report-error-state";
import { ReportLoadingState } from "@/components/report/report-loading-state";
import { ReportOverviewCard } from "@/components/report/report-overview-card";
import { Button } from "@/components/ui/button";
import { Divider } from "@/components/ui/divider";
import { Heading } from "@/components/ui/heading";
import { useCopyToClipboard } from "@/hooks/use-copy-to-clipboard";
import { useReport } from "@/hooks/use-report";
import { hasOverviewData } from "@/lib/report/overview-metrics";
import type { RawOverview, RawReportPayload } from "@/lib/types/raw-report";
import type { ReportEntry } from "@/lib/types/report";

type ReportViewProps = {
  hash: string;
};

export function ReportView({ hash }: ReportViewProps) {
  const t = useTranslations("report");
  const tCommon = useTranslations("common");
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
        <Heading>{t("title")}</Heading>
        <div className="flex items-center gap-2">
          <Button className="-my-0.5" disabled={isCopied} onClick={handleShare}>
            {isCopied ? tCommon("actions.copied") : tCommon("actions.share")}
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
            {entries.map((entry, index) => {
              const payload = (entry.report ?? {}) as RawReportPayload;
              const emailTime = payload?.metadata?.email_time;
              const entryKey = `${emailTime ?? entry.startDate ?? index}-${index}`;

              return (
                <div key={entryKey}>
                  <ReportEntryCard entry={entry} />
                  {index < entries.length - 1 ? <Divider className="my-8" /> : null}
                </div>
              );
            })}
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
