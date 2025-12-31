"use client";

import { FunnelIcon } from "@heroicons/react/16/solid";
import { useTranslations } from "next-intl";
import { useContext } from "react";
import { GovernorContext } from "@/components/context/GovernorContext";
import { BattleLog } from "@/components/reports/BattleLog";
import { ReportsFilterDialog } from "@/components/reports/ReportsFilterDialog";
import ReportsTable from "@/components/reports/ReportsTable";
import { Heading, Subheading } from "@/components/ui/Heading";
import { Table, TableHead, TableHeader, TableRow } from "@/components/ui/Table";
import { useCurrentUser } from "@/hooks/useCurrentUser";

export default function Page() {
  const tAccount = useTranslations("account");
  const tReports = useTranslations("reports");
  const tCommon = useTranslations("common");
  const { user, loading } = useCurrentUser();
  const governorContext = useContext(GovernorContext);

  if (!governorContext) {
    throw new Error("My Reports page must be used within a GovernorProvider");
  }

  const { activeGovernor } = governorContext;

  if (loading) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400" role="status" aria-live="polite">
        {tAccount("states.loading")}
      </p>
    );
  }

  if (!user) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400" role="status" aria-live="polite">
        {tAccount("states.loginRequired")}
      </p>
    );
  }

  if (!activeGovernor) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400" role="status" aria-live="polite">
        {tAccount("states.governorRequired")}
      </p>
    );
  }

  return (
    <>
      <Heading>{tAccount("titles.reports")}</Heading>
      <BattleLog governorId={activeGovernor.governorId} year={2025} />
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
