"use client";

import FavoriteReportsTable from "@/components/reports/FavoriteReportsTable";
import { Heading } from "@/components/ui/Heading";
import { Table, TableHead, TableHeader, TableRow } from "@/components/ui/Table";
import { useCurrentUser } from "@/hooks/useCurrentUser";

export default function Page() {
  const { user, loading } = useCurrentUser();

  if (loading) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400" role="status" aria-live="polite">
        Loading your account&hellip;
      </p>
    );
  }

  if (!user) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400" role="status" aria-live="polite">
        You must be logged in to view this page.
      </p>
    );
  }

  return (
    <>
      <Heading>My Favorite Reports</Heading>
      <Table dense className="mt-4 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
        <TableHead>
          <TableRow>
            <TableHeader className="sm:w-1/6">Time</TableHeader>
            <TableHeader>Sender</TableHeader>
            <TableHeader>Opponent</TableHeader>
            <TableHeader className="sm:w-1/6">Battles</TableHeader>
            <TableHeader className="sm:w-1/6">Duration</TableHeader>
          </TableRow>
        </TableHead>
        <FavoriteReportsTable />
      </Table>
    </>
  );
}
