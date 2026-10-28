"use client";

import { Funnel } from "lucide-react";

import { use } from "react";
import { ReportsFilterDialog } from "@/components/reports/reports-filter-dialog";
import ReportsTable from "@/components/reports/reports-table";
import { Heading, Subheading } from "@/components/ui/heading";
import { GovernorContext } from "@/providers/governor-context";

export function AccountReportsContent() {
  const governorContext = use(GovernorContext);

  if (!governorContext) {
    throw new Error("My Reports page must be used within a GovernorProvider");
  }

  const { activeGovernor } = governorContext;

  if (!activeGovernor) {
    return null;
  }

  return (
    <>
      <Heading>My Battle Reports</Heading>
      <div className="mt-8 flex items-end justify-between">
        <Subheading>Live feed (UTC)</Subheading>
        <ReportsFilterDialog lockedPlayerId={activeGovernor.governorId}>
          <Funnel data-slot="icon" aria-hidden="true" />
          Filter
        </ReportsFilterDialog>
      </div>
      <ReportsTable scope="mine" />
    </>
  );
}
