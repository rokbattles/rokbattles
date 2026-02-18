"use client";

import { FunnelIcon } from "@heroicons/react/16/solid";
import { useExtracted } from "next-intl";
import { useContext } from "react";
import { ReportsFilterDialog } from "@/components/reports/reports-filter-dialog";
import ReportsTable from "@/components/reports/reports-table";
import { Heading, Subheading } from "@/components/ui/heading";
import { Table, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { GovernorContext } from "@/providers/governor-context";

export function AccountReportsContent() {
  const tAccount = useExtracted();
  const tReports = useExtracted();
  const tCommon = useExtracted();
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
      <Heading>{tAccount("My Battle Reports")}</Heading>
      <div className="mt-8 flex items-end justify-between">
        <Subheading>{tCommon("Live feed (UTC)")}</Subheading>
        <ReportsFilterDialog lockedPlayerId={activeGovernor.governorId}>
          <FunnelIcon />
          {tReports("Filter")}
        </ReportsFilterDialog>
      </div>
      <Table dense grid className="mt-4 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
        <TableHead>
          <TableRow>
            <TableHeader className="sm:w-36">{tCommon("Time")}</TableHeader>
            <TableHeader>{tCommon("Sender")}</TableHeader>
            <TableHeader>{tCommon("Opponent")}</TableHeader>
            <TableHeader className="sm:w-32">{tCommon("Battles")}</TableHeader>
            <TableHeader className="sm:w-32">{tReports("Kill Count")}</TableHeader>
            <TableHeader className="sm:w-32">{tReports("Trade %")}</TableHeader>
            <TableHeader className="sm:w-32">{tReports("Duration")}</TableHeader>
          </TableRow>
        </TableHead>
        <ReportsTable scope="mine" />
      </Table>
    </>
  );
}
