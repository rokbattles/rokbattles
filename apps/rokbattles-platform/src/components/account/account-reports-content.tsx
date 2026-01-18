"use client";

import { FunnelIcon } from "@heroicons/react/16/solid";
import { useTranslations } from "next-intl";
import { useContext } from "react";
import { BattleLog } from "@/components/reports/battle-log";
import { ReportsFilterDialog } from "@/components/reports/reports-filter-dialog";
import ReportsTable from "@/components/reports/reports-table";
import { Heading, Subheading } from "@/components/ui/heading";
import { Table, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { GovernorContext } from "@/providers/governor-context";

export function AccountReportsContent() {
  const tAccount = useTranslations("account");
  const tReports = useTranslations("reports");
  const tCommon = useTranslations("common");
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
      <Heading>{tAccount("titles.reports")}</Heading>
      <BattleLog governorId={activeGovernor.governorId} />
      <div className="mt-8 flex items-end justify-between">
        <Subheading>{tCommon("headings.liveFeed")}</Subheading>
        <ReportsFilterDialog lockedPlayerId={activeGovernor.governorId}>
          <FunnelIcon />
          {tReports("filter.trigger")}
        </ReportsFilterDialog>
      </div>
      <Table dense className="mt-4 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
        <TableHead>
          <TableRow>
            <TableHeader className="sm:w-1/6">{tCommon("labels.time")}</TableHeader>
            <TableHeader>{tCommon("labels.sender")}</TableHeader>
            <TableHeader>{tCommon("labels.opponent")}</TableHeader>
            <TableHeader className="sm:w-1/6">{tCommon("labels.battles")}</TableHeader>
            <TableHeader className="sm:w-1/6">{tReports("table.duration")}</TableHeader>
          </TableRow>
        </TableHead>
        <ReportsTable scope="mine" />
      </Table>
    </>
  );
}
