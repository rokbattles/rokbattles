"use client";

import { cn } from "cn";
import { useSearchParams } from "next/navigation";
import { useExtracted } from "next-intl";
import { type ReactElement, use, useState } from "react";
import { ArkBattleReports } from "@/components/account-ark/ark-battle-reports";
import { ArkMatchDetailIndividualResultsSection } from "@/components/account-ark/ark-match-detail-individual-results-section";
import { ArkMatchDetailPairingsSection } from "@/components/account-ark/ark-match-detail-pairings-section";
import { ArkScoreboard } from "@/components/account-ark/ark-scoreboard";
import { ArkRequestState } from "@/components/account-ark/ark-shared";
import { ArkTeamScores } from "@/components/account-ark/ark-team-scores";
import { useArkQuery } from "@/hooks/use-ark-query";
import type { ArkMatchDetail, ArkMatchDetailResponse } from "@/lib/types/ark";
import { GovernorContext } from "@/providers/governor-context";

type ArkMatchDetailContentProps = { matchId: string };

type ArkMatchDashboardProps = {
  detail: ArkMatchDetail;
  governorId: number;
};

const TABS = ["overview", "team", "pairings", "reports"] as const;
type ArkTab = (typeof TABS)[number];

export function ArkMatchDetailContent({ matchId }: ArkMatchDetailContentProps): ReactElement {
  const t = useExtracted();
  const context = use(GovernorContext);

  if (!context) {
    throw new Error("Ark Recap must be used within a GovernorProvider");
  }

  const governorId = context.activeGovernor?.governorId;
  const query = useArkQuery<ArkMatchDetailResponse>(
    governorId ? `/proxy/v1/governor/${governorId}/ark/${encodeURIComponent(matchId)}` : null
  );

  let content: ReactElement;

  if (query.loading) {
    content = <ArkRequestState>{t("Loading Ark recap…")}</ArkRequestState>;
  } else if (query.error) {
    content = (
      <ArkRequestState error retry={query.retry}>
        {t("Failed to load this Ark recap.")}
      </ArkRequestState>
    );
  } else if (!query.data?.match || !governorId) {
    content = (
      <ArkRequestState>
        {t("This match is not available for the selected governor.")}
      </ArkRequestState>
    );
  } else {
    content = (
      <ArkMatchDashboard
        key={`${governorId}:${matchId}`}
        detail={query.data.match}
        governorId={governorId}
      />
    );
  }

  return <div className="mt-4">{content}</div>;
}

function ArkMatchDashboard({ detail, governorId }: ArkMatchDashboardProps): ReactElement {
  const t = useExtracted();
  const params = useSearchParams();

  const [tab, setTab] = useState<ArkTab>(
    () => TABS.find((value) => value === params.get("tab")) ?? "overview"
  );

  const labels = {
    overview: t("Overview"),
    team: t("Team scores"),
    pairings: t("Pairings"),
    reports: t("Battle reports"),
  };
  const counts = {
    overview: null,
    team: detail.participants.length,
    pairings: detail.pairings.length,
    reports: detail.battleReports.total,
  };

  let content: ReactElement;

  switch (tab) {
    case "team":
      content = <ArkTeamScores participants={detail.participants} />;
      break;
    case "pairings":
      content = <ArkMatchDetailPairingsSection pairings={detail.pairings} />;
      break;
    case "reports":
      content = <ArkBattleReports governorId={governorId} detail={detail} />;
      break;
    default:
      content = <ArkMatchDetailIndividualResultsSection detail={detail} />;
  }

  return (
    <div className="space-y-6">
      <ArkScoreboard detail={detail} />
      <nav
        aria-label={t("Match sections")}
        className="flex gap-6 overflow-x-auto border-b border-zinc-200 dark:border-zinc-800"
      >
        {TABS.map((value) => (
          <button
            key={value}
            type="button"
            aria-pressed={tab === value}
            onClick={() => setTab(value)}
            className={cn(
              "flex shrink-0 items-center gap-2 border-b-2 pb-3 text-sm font-medium",
              "focus-visible:outline-2 focus-visible:outline-blue-500",
              tab === value
                ? "border-zinc-950 text-zinc-950 dark:border-white dark:text-white"
                : "border-transparent text-zinc-500 dark:text-zinc-400"
            )}
          >
            {labels[value]}
            {counts[value] != null ? (
              <span className="rounded bg-zinc-100 px-1.5 py-0.5 text-xs tabular-nums dark:bg-zinc-800">
                {counts[value]}
              </span>
            ) : null}
          </button>
        ))}
      </nav>
      {content}
    </div>
  );
}
