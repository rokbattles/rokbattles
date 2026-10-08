"use client";

import { useExtracted } from "next-intl";
import { type ReactElement, use } from "react";
import { ArkHistoryDashboard } from "@/components/account-ark/ark-history-dashboard";
import { ArkRequestState } from "@/components/account-ark/ark-shared";
import { useArkQuery } from "@/hooks/use-ark-query";
import type { ArkMatchHistoryResult } from "@/lib/types/ark";
import { GovernorContext } from "@/providers/governor-context";

export function ArkMatchHistoryContent(): ReactElement {
  const t = useExtracted();
  const context = use(GovernorContext);

  if (!context) {
    throw new Error("Ark Recap must be used within a GovernorProvider");
  }

  const governorId = context.activeGovernor?.governorId;
  const query = useArkQuery<ArkMatchHistoryResult>(
    governorId ? `/proxy/v1/governor/${governorId}/ark?limit=250` : null
  );

  if (query.loading) {
    return <ArkRequestState>{t("Loading Ark matches…")}</ArkRequestState>;
  }

  if (query.error) {
    return (
      <ArkRequestState error retry={query.retry}>
        {t("Failed to load Ark matches.")}
      </ArkRequestState>
    );
  }

  if (!query.data) {
    return (
      <ArkRequestState>
        {t("No Ark matches yet. Upload your match result mails to start your recap.")}
      </ArkRequestState>
    );
  }

  return <ArkHistoryDashboard key={governorId} data={query.data} />;
}
