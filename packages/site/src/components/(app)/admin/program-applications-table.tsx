"use client";

import { useState } from "react";
import { ProgramApplicationReviewDrawer } from "@/components/(app)/admin/program-application-review-drawer";
import { ProgramApplicationStatus } from "@/components/(app)/admin/program-application-status";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  type AdminApplicationDetail,
  type AdminApplicationListItem,
  type AdminApplicationSummary,
  fetchAdminApplication,
} from "@/lib/(app)/admin-program-applications";
import { getProgramLabel } from "@/lib/(app)/program-applications";
import { formatCompactUtcDateTime } from "@/lib/datetime";

type SelectedApplication = {
  summary: AdminApplicationSummary;
  detail?: AdminApplicationDetail | null;
};

export function ProgramApplicationsTable({
  applications,
}: {
  applications: AdminApplicationListItem[];
}) {
  const [selected, setSelected] = useState<SelectedApplication | null>(null);

  async function openApplication(summary: AdminApplicationSummary) {
    const selection = { summary };
    setSelected(selection);

    const detail = await fetchAdminApplication(summary.id).catch(() => null);

    setSelected((current) => (current === selection ? { summary, detail } : current));
  }

  return (
    <>
      <Table className="[&_table]:w-full [&_table]:min-w-[55rem] [&_table]:table-fixed">
        <colgroup>
          <col className="w-[16%]" />
          <col className="w-[24%]" />
          <col className="w-[22%]" />
          <col className="w-[20%]" />
          <col className="w-[18%]" />
        </colgroup>
        <TableHead>
          <TableRow>
            <TableHeader>Time</TableHeader>
            <TableHeader>Governor</TableHeader>
            <TableHeader>Discord</TableHeader>
            <TableHeader>Program</TableHeader>
            <TableHeader>Status</TableHeader>
          </TableRow>
        </TableHead>
        <TableBody>
          {applications.map((application) => {
            const timestamp = new Date(application.createdAt).toISOString();
            const governor = application.defaultGovernor;

            return (
              <TableRow
                key={application.id}
                className="relative hover:bg-white/5 focus-within:bg-white/5 [&>td:first-child]:static"
              >
                <TableCell>
                  <time dateTime={timestamp}>
                    {formatCompactUtcDateTime(application.createdAt)}
                  </time>
                  <button
                    type="button"
                    className="absolute inset-0 z-10 cursor-pointer focus:outline-hidden focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-inset"
                    onClick={() => openApplication(application)}
                    aria-label={`View application for ${application.discordId}`}
                    aria-haspopup="dialog"
                  />
                </TableCell>
                <TableCell className="truncate">
                  {governor?.governorName || governor?.governorId || "None"}
                </TableCell>
                <TableCell>{application.discordId}</TableCell>
                <TableCell>{getProgramLabel(application.program)}</TableCell>
                <TableCell>
                  <ProgramApplicationStatus status={application.status} />
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
      {selected ? (
        <ProgramApplicationReviewDrawer
          key={selected.summary.id}
          summary={selected.summary}
          detail={selected.detail}
          onClose={() => setSelected(null)}
        />
      ) : null}
    </>
  );
}
