"use client";

import { useRouter } from "next/navigation";
import { useState, useTransition } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Drawer,
  DrawerActions,
  DrawerBody,
  DrawerDescription,
  DrawerTitle,
} from "@/components/ui/drawer";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Text } from "@/components/ui/text";
import {
  ADMIN_PROGRAM_APPLICATIONS_PATH,
  type AdminApplicationDetail,
  type AdminApplicationListItem,
  type AdminApplicationSummary,
} from "@/lib/(app)/admin-program-applications";
import { PROGRAMS } from "@/lib/(app)/program-applications";
import type { Program } from "@/lib/(app)/types/program-application";

function formatSubmittedTime(iso: string) {
  return `${iso.slice(5, 10).replace("-", "/")} ${iso.slice(11, 16)} UTC`;
}

const programLabel = (program: Program) =>
  PROGRAMS.find(({ id }) => id === program)?.label ?? program;

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
    const detail = await fetchApplication(applicationEndpoint(summary.id)).catch(() => null);
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
                  <time dateTime={timestamp}>{formatSubmittedTime(timestamp)}</time>
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
                <TableCell>{programLabel(application.program)}</TableCell>
                <TableCell>
                  <ApplicationStatus status={application.status} />
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
      {selected ? (
        <ApplicationReviewDrawer
          key={selected.summary.id}
          selection={selected}
          onClose={() => setSelected(null)}
        />
      ) : null}
    </>
  );
}

function ApplicationStatus({ status }: { status: string }) {
  switch (status) {
    case "open":
      return <Badge color="amber">Open</Badge>;
    case "approving":
      return <Badge color="blue">Approval in progress</Badge>;
    case "approved":
      return <Badge color="green">Approved</Badge>;
    case "closed":
      return <Badge>Closed</Badge>;
    default:
      return <Badge>{status}</Badge>;
  }
}

function applicationEndpoint(id: string) {
  return `/proxy${ADMIN_PROGRAM_APPLICATIONS_PATH}/${id}`;
}

async function fetchApplication(endpoint: string) {
  const response = await fetch(endpoint, { cache: "no-store" });
  if (!response.ok) throw new Error("Application details unavailable");
  return (await response.json()) as AdminApplicationDetail;
}

function ApplicationReviewDrawer({
  selection: { summary, detail },
  onClose,
}: {
  selection: SelectedApplication;
  onClose: () => void;
}) {
  const router = useRouter();
  const endpoint = applicationEndpoint(summary.id);
  const [reviewedApplication, setReviewedApplication] = useState<AdminApplicationDetail | null>(
    null
  );
  const [reviewError, setReviewError] = useState("");
  const [pending, startTransition] = useTransition();
  const application = reviewedApplication ?? detail;
  const status = application?.status ?? summary.status;

  function review(decision: "approve" | "deny") {
    if (!application || pending) return;
    setReviewError("");
    startTransition(async () => {
      try {
        const response = await fetch(endpoint, {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ decision }),
        });
        if (response.ok) {
          const reviewed = (await response.json()) as AdminApplicationSummary;
          setReviewedApplication({ ...application, ...reviewed });
          router.refresh();
          return;
        }

        if (response.status === 409) {
          setReviewError("This application was updated by another request.");
          setReviewedApplication(await fetchApplication(endpoint));
          router.refresh();
          return;
        }
      } catch {
        // Keep the application open for another review attempt.
      }
      setReviewError("Unable to save the decision. Try again.");
    });
  }

  return (
    <Drawer open side="right" size="xl" onClose={() => !pending && onClose()}>
      <DrawerTitle>{programLabel(summary.program)} Application</DrawerTitle>
      <DrawerDescription>
        Submitted {formatSubmittedTime(new Date(summary.createdAt).toISOString())}
      </DrawerDescription>
      <DrawerBody className="space-y-6">
        <div aria-live="polite">
          <ApplicationStatus status={status} />
        </div>
        <ApplicationDetails application={application} />
        {reviewError ? (
          <p role="alert" className="text-sm text-red-400">
            {reviewError}
          </p>
        ) : null}
      </DrawerBody>
      <DrawerActions>
        <Button plain disabled={pending} onClick={onClose}>
          Close
        </Button>
        {application && status === "open" ? (
          <Button outline disabled={pending} onClick={() => review("deny")}>
            Deny
          </Button>
        ) : null}
        {application && (status === "open" || status === "approving") ? (
          <Button disabled={pending} onClick={() => review("approve")}>
            {status === "approving" ? "Retry approval" : "Approve"}
          </Button>
        ) : null}
      </DrawerActions>
    </Drawer>
  );
}

function ApplicationDetails({
  application,
}: {
  application: AdminApplicationDetail | null | undefined;
}) {
  if (application === undefined) {
    return <Text role="status">Loading application details…</Text>;
  }
  if (!application) {
    return <Text role="alert">Unable to load application details. Close and reopen to retry.</Text>;
  }

  const governor = application.defaultGovernor;
  const governorLabel = governor?.governorName
    ? `${governor.governorName} (${governor.governorId})`
    : governor?.governorId;

  return (
    <>
      <dl className="space-y-3 text-sm">
        <div>
          <dt className="text-zinc-400">Discord ID</dt>
          <dd>{application.discordId}</dd>
        </div>
        <div>
          <dt className="text-zinc-400">Email</dt>
          <dd className="wrap-anywhere">{application.email ?? "Unavailable"}</dd>
        </div>
        <div>
          <dt className="text-zinc-400">Default governor</dt>
          <dd>{governorLabel ?? "None"}</dd>
        </div>
      </dl>
      <dl className="space-y-4 border-t border-white/10 pt-5">
        {application.form.map(({ question, answer }) => (
          <div key={question} className="space-y-1">
            <dt className="text-sm font-semibold">{question}</dt>
            <dd className="whitespace-pre-wrap wrap-anywhere text-sm/6 text-zinc-300">{answer}</dd>
          </div>
        ))}
      </dl>
    </>
  );
}
