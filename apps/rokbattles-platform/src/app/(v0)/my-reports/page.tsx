"use client";

import { FunnelIcon } from "@heroicons/react/16/solid";
import { useContext } from "react";
import { GovernorContext } from "@/components/context/GovernorContext";
import { BattleLog } from "@/components/reports/BattleLog";
import { MyReportsFilterDialog } from "@/components/reports/MyReportsFilterDialog";
import ReportsTable from "@/components/reports/ReportsTable";
import { Heading, Subheading } from "@/components/ui/Heading";
import { Table, TableHead, TableHeader, TableRow } from "@/components/ui/Table";
import { useCurrentUser } from "@/hooks/useCurrentUser";

export default function Page() {
  const { user, loading } = useCurrentUser();
  const governorContext = useContext(GovernorContext);

  if (!governorContext) {
    throw new Error("My Reports page must be used within a GovernorProvider");
  }

  const { activeGovernor } = governorContext;

  if (loading) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400">Loading your account&hellip;</p>
    );
  }

  if (!user) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400">
        You must be logged in to view this page.
      </p>
    );
  }

  if (!activeGovernor) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400">
        You must have a claimed governor to view this page.
      </p>
    );
  }

  return (
    <>
      <Heading>My Battle Reports</Heading>
      <BattleLog governorId={activeGovernor.governorId} year={2025} />
      <div className="mt-8 flex items-end justify-between">
        <Subheading>Live feed (UTC)</Subheading>
        <MyReportsFilterDialog>
          <FunnelIcon />
          Filter
        </MyReportsFilterDialog>
      </div>
      <Table dense className="mt-4 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
        <TableHead>
          <TableRow>
            <TableHeader className="sm:w-1/6">Time</TableHeader>
            <TableHeader>Self Participant</TableHeader>
            <TableHeader>Enemy Participant</TableHeader>
            <TableHeader className="sm:w-1/6">Battles</TableHeader>
            <TableHeader className="sm:w-1/6">Duration</TableHeader>
          </TableRow>
        </TableHead>
        <ReportsTable scope="mine" />
      </Table>
    </>
  );
}
