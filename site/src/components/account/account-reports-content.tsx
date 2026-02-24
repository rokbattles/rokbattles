"use client";

import { FunnelIcon } from "@heroicons/react/16/solid";
import { useExtracted } from "next-intl";
import { useContext } from "react";
import { ReportsFilterDialog } from "@/components/reports/reports-filter-dialog";
import ReportsTable from "@/components/reports/reports-table";
import { Heading, Subheading } from "@/components/ui/heading";
import { GovernorContext } from "@/providers/governor-context";

export function AccountReportsContent() {
  const t = useExtracted();
  const governorContext = useContext(GovernorContext);

  if (!governorContext) {
    throw new Error("My Reports page must be used within a GovernorProvider");
  }

  const { activeGovernor } = governorContext;

  if (!activeGovernor) {
    return null;
  }

  return (
    <>
      <Heading>{t("My Battle Reports")}</Heading>
      <div className="mt-8 flex items-end justify-between">
        <Subheading>{t("Live feed (UTC)")}</Subheading>
        <ReportsFilterDialog lockedPlayerId={activeGovernor.governorId}>
          <FunnelIcon />
          {t("Filter")}
        </ReportsFilterDialog>
      </div>
      <ReportsTable scope="mine" />
    </>
  );
}
