"use client";

import { useContext } from "react";
import { ArkMatchHistoryEmptyState } from "@/components/account-ark/ark-match-history-empty-state";
import { ArkMatchHistoryErrorState } from "@/components/account-ark/ark-match-history-error-state";
import { ArkMatchHistoryLoadingState } from "@/components/account-ark/ark-match-history-loading-state";
import { ArkMatchHistoryTable } from "@/components/account-ark/ark-match-history-table";
import { useArkMatchHistory } from "@/hooks/use-ark-match-history";
import { GovernorContext } from "@/providers/governor-context";

type ArkMatchHistoryContentProps = {
  limit?: number;
};

export function ArkMatchHistoryContent({ limit }: ArkMatchHistoryContentProps) {
  const governorContext = useContext(GovernorContext);
  if (!governorContext) {
    throw new Error("Ark match history must be used within a GovernorProvider");
  }

  const governorId = governorContext.activeGovernor?.governorId;
  const { data, loading, error } = useArkMatchHistory({ governorId, limit });

  if (loading) {
    return <ArkMatchHistoryLoadingState />;
  }

  if (error) {
    return <ArkMatchHistoryErrorState />;
  }

  if (!data || data.items.length === 0) {
    return <ArkMatchHistoryEmptyState />;
  }

  return (
    <section className="mt-8 space-y-4">
      <ArkMatchHistoryTable rows={data.items} />
    </section>
  );
}
