"use client";

import { Funnel } from "lucide-react";

import { use } from "react";
import { ReportsFilterDialog } from "@/components/reports/reports-filter-dialog";
import ReportsTable from "@/components/reports/reports-table";
import { Heading, Subheading } from "@/components/ui/heading";
import type { ReportsBattleKind } from "@/lib/reports-query";
import { GovernorContext } from "@/providers/governor-context";

type AccountReportsContentProps = {
  title?: string;
  /** Limits the list to open field battles, or to rallies and garrisons. */
  kind?: ReportsBattleKind;
};

export function AccountReportsContent({
  title = "Battle Reports",
  kind,
}: AccountReportsContentProps) {
  const governorContext = use(GovernorContext);

  if (!governorContext) {
    throw new Error("Reports page must be used within a GovernorProvider");
  }

  const { activeGovernor } = governorContext;

  if (!activeGovernor) {
    return null;
  }

  return (
    <>
      <Heading>{title}</Heading>
      <div className="mt-8 flex items-end justify-between">
        <Subheading>Live feed (UTC)</Subheading>
        <ReportsFilterDialog
          lockedPlayerId={activeGovernor.governorId}
          hideRallyGarrison={kind !== undefined}
        >
          <Funnel data-slot="icon" aria-hidden="true" />
          Filter
        </ReportsFilterDialog>
      </div>
      <ReportsTable scope="mine" kind={kind} />
    </>
  );
}
