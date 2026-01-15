"use client";

import { StarIcon } from "@heroicons/react/16/solid";
import { useSearchParams } from "next/navigation";
import { useTranslations } from "next-intl";
import { useEffect, useRef, useState } from "react";
import { ReportEmptyState } from "@/components/report/report-empty-state";
import { ReportEntryCard } from "@/components/report/report-entry-card";
import { ReportErrorState } from "@/components/report/report-error-state";
import { ReportLoadingState } from "@/components/report/report-loading-state";
import { ReportOverviewCard } from "@/components/report/report-overview-card";
import { Button } from "@/components/ui/Button";
import { Divider } from "@/components/ui/Divider";
import { Heading } from "@/components/ui/Heading";
import {
  Sidebar,
  SidebarBody,
  SidebarItem,
  SidebarLabel,
  SidebarSection,
} from "@/components/ui/Sidebar";
import { Text } from "@/components/ui/Text";
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
  mergeMode?: boolean;
};

export function ReportView({ hash, mergeMode = false }: ReportViewProps) {
  const t = useTranslations("report");
  const tCommon = useTranslations("common");
  const searchParams = useSearchParams();
  const normalizedHash = hash?.trim() ?? "";
  const searchParamsString = searchParams.toString();

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
  const mergeReports = [...(data?.merge?.reports ?? [])].sort(
    (a, b) => b.latestEmailTime - a.latestEmailTime
  );
  const mergeEligible = mergeReports.length > 1;
  const showMergeBanner = mergeEligible;
  const showMergeLayout = mergeEligible && mergeMode;
  const mergeToggleHref = buildReportHref(normalizedHash, searchParamsString, !mergeMode);
  const mergeActionLabel = mergeMode
    ? t("merge.bannerActionDisable")
    : t("merge.bannerActionEnable");
  const mergeTitle = mergeMode ? t("merge.bannerTitleEnable") : t("merge.bannerTitleDisable");

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

  const reportContent = (
    <section className="space-y-8">
      {showMergeBanner ? (
        <div className="rounded-md bg-blue-50 p-4 dark:bg-blue-500/10 dark:outline dark:outline-blue-500/20">
          <div className="flex items-center">
            <div className="ml-3 flex-1 md:flex md:items-center md:justify-between">
              <Text className="text-blue-700! dark:text-blue-300!">{mergeTitle}</Text>
              <div className="mt-3 text-sm md:mt-0 md:ml-6">
                <Button color="blue" href={mergeToggleHref}>
                  {mergeActionLabel}
                </Button>
              </div>
            </div>
          </div>
        </div>
      ) : null}
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

  if (!showMergeLayout) {
    return reportContent;
  }

  return (
    <div className="grid gap-2 lg:grid-cols-[220px_minmax(0,1fr)]">
      <aside>
        <Sidebar>
          <SidebarBody className="p-0">
            <SidebarSection>
              {mergeReports.map((report, index) => {
                const labelNumber = mergeReports.length - index;
                const href = buildReportHref(report.parentHash, searchParamsString, true);
                const isActive = report.parentHash === normalizedHash;
                return (
                  <SidebarItem key={report.parentHash} href={href} current={isActive}>
                    <SidebarLabel>{t("merge.sidebarItem", { index: labelNumber })}</SidebarLabel>
                  </SidebarItem>
                );
              })}
            </SidebarSection>
          </SidebarBody>
        </Sidebar>
      </aside>
      {reportContent}
    </div>
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

function buildReportHref(hash: string, queryString: string, mergeEnabled: boolean) {
  const params = new URLSearchParams(queryString);
  if (mergeEnabled) {
    params.set("merge", "1");
  } else {
    params.delete("merge");
  }
  const query = params.toString();
  return `/report/${encodeURIComponent(hash)}${query ? `?${query}` : ""}`;
}
