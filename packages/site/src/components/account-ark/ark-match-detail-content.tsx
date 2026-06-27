"use client";

import { use } from "react";
import { ArkMatchDetailErrorState } from "@/components/account-ark/ark-match-detail-error-state";
import { ArkMatchDetailIndividualResultsSection } from "@/components/account-ark/ark-match-detail-individual-results-section";
import { ArkMatchDetailLoadingState } from "@/components/account-ark/ark-match-detail-loading-state";
import { ArkMatchDetailNotFoundState } from "@/components/account-ark/ark-match-detail-not-found-state";
import { ArkMatchDetailOverviewSection } from "@/components/account-ark/ark-match-detail-overview-section";
import { ArkMatchDetailPairingsSection } from "@/components/account-ark/ark-match-detail-pairings-section";
import { useArkMatchDetail } from "@/hooks/use-ark-match-detail";
import { GovernorContext } from "@/providers/governor-context";

type ArkMatchDetailContentProps = {
  matchId: string;
};

export function ArkMatchDetailContent({ matchId }: ArkMatchDetailContentProps) {
  const governorContext = use(GovernorContext);
  if (!governorContext) {
    throw new Error("Ark match details must be used within a GovernorProvider");
  }

  const governorId = governorContext.activeGovernor?.governorId;
  const { data, loading, error } = useArkMatchDetail({ governorId, matchId });

  if (loading) {
    return <ArkMatchDetailLoadingState />;
  }

  if (error) {
    return <ArkMatchDetailErrorState />;
  }

  const detail = data?.match;
  if (!detail) {
    return <ArkMatchDetailNotFoundState />;
  }

  return (
    <section className="mt-8 space-y-6">
      <ArkMatchDetailOverviewSection overview={detail.overview} />
      <ArkMatchDetailIndividualResultsSection individualResults={detail.individualResults} />
      <ArkMatchDetailPairingsSection pairings={detail.pairings} />
    </section>
  );
}
