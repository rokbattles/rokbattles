import { useRouter } from "next/navigation";
import { useState, useTransition } from "react";
import { ProgramApplicationDetails } from "@/components/(app)/admin/program-application-details";
import { ProgramApplicationStatus } from "@/components/(app)/admin/program-application-status";
import { Button } from "@/components/ui/button";
import {
  Drawer,
  DrawerActions,
  DrawerBody,
  DrawerDescription,
  DrawerTitle,
} from "@/components/ui/drawer";
import {
  type AdminApplicationDetail,
  type AdminApplicationSummary,
  type ApplicationDecision,
  fetchAdminApplication,
  reviewAdminApplication,
} from "@/lib/(app)/admin-program-applications";
import { getProgramLabel } from "@/lib/(app)/program-applications";
import { formatCompactUtcDateTime } from "@/lib/datetime";

export function ProgramApplicationReviewDrawer({
  summary,
  detail,
  onClose,
}: {
  summary: AdminApplicationSummary;
  detail?: AdminApplicationDetail | null;
  onClose: () => void;
}) {
  const router = useRouter();
  const [reviewedApplication, setReviewedApplication] = useState<AdminApplicationDetail | null>(
    null
  );
  const [reviewError, setReviewError] = useState("");
  const [pending, startTransition] = useTransition();

  const application = reviewedApplication ?? detail;
  const status = application?.status ?? summary.status;

  function review(decision: ApplicationDecision) {
    if (!application || pending) {
      return;
    }

    setReviewError("");

    startTransition(async () => {
      try {
        const reviewed = await reviewAdminApplication(summary.id, decision);

        if (reviewed) {
          setReviewedApplication({ ...application, ...reviewed });
        } else {
          setReviewedApplication(await fetchAdminApplication(summary.id));
          setReviewError("This application was updated by another request.");
        }

        router.refresh();
      } catch {
        setReviewError("Unable to save the decision. Try again.");
      }
    });
  }

  return (
    <Drawer open side="right" size="xl" onClose={() => !pending && onClose()}>
      <DrawerTitle>{getProgramLabel(summary.program)} Application</DrawerTitle>
      <DrawerDescription>Submitted {formatCompactUtcDateTime(summary.createdAt)}</DrawerDescription>
      <DrawerBody className="space-y-6">
        <div aria-live="polite">
          <ProgramApplicationStatus status={status} />
        </div>
        <ProgramApplicationDetails application={application} />
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
