"use client";

import { useTranslations } from "next-intl";
import FavoriteReportsTable from "@/components/reports/FavoriteReportsTable";
import { Heading } from "@/components/ui/Heading";
import { Table, TableHead, TableHeader, TableRow } from "@/components/ui/Table";
import { useCurrentUser } from "@/hooks/useCurrentUser";

export default function Page() {
  const tAccount = useTranslations("account");
  const tReports = useTranslations("reports");
  const tCommon = useTranslations("common");
  const { user, loading } = useCurrentUser();

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

  return (
    <>
      <Heading>{tAccount("titles.favorites")}</Heading>
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
        <FavoriteReportsTable />
      </Table>
    </>
  );
}
