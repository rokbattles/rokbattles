"use client";

import { StarIcon } from "@heroicons/react/16/solid";
import { useTranslations } from "next-intl";
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
import { useCurrentUser } from "@/hooks/useCurrentUser";
import { useReport } from "@/hooks/useReport";
import { useReportFavorite } from "@/hooks/useReportFavorite";
import { cn } from "@/lib/cn";
import { hasOverviewData } from "@/lib/report/overviewMetrics";
import type { RawOverview, RawReportPayload } from "@/lib/types/rawReport";
import type { ReportEntry } from "@/lib/types/report";

type ReportViewProps = {
  hash: string;
};

export function ReportView({ hash }: ReportViewProps) {
  const t = useTranslations("report");
  const tCommon = useTranslations("common");
  const normalizedHash = hash?.trim() ?? "";

  const { data, loading, error } = useReport(normalizedHash.length > 0 ? normalizedHash : null);
  const { user, loading: userLoading } = useCurrentUser();
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
  const showFavoriteButton = Boolean(!userLoading && user && normalizedHash.length > 0);

  const {
    favorited,
    loading: favoriteLoading,
    updating,
    toggleFavorite,
  } = useReportFavorite({
    parentHash: normalizedHash,
    reportType: "battle",
    enabled: showFavoriteButton,
  });

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
          {showFavoriteButton ? (
            <Button
              className="-my-0.5"
              aria-label={favorited ? t("actions.unfavoriteAria") : t("actions.favoriteAria")}
              aria-pressed={favorited}
              disabled={favoriteLoading || updating}
              onClick={toggleFavorite}
            >
              <StarIcon data-slot="icon" className={cn(favorited ? "fill-amber-500" : "")} />
              {favorited ? tCommon("actions.unfavorite") : tCommon("actions.favorite")}
            </Button>
          ) : null}
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
