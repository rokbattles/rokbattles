"use client";

import { usePathname, useSearchParams } from "next/navigation";

import { Button } from "@/components/ui/button";
import {
  Drawer,
  DrawerActions,
  DrawerBody,
  DrawerDescription,
  DrawerTitle,
} from "@/components/ui/drawer";
import { Subheading } from "@/components/ui/heading";
import type { ReportsListItem } from "@/lib/types/reports-list";
import ReportsOverviewSummaryCard from "./reports-overview-summary-card";
import ReportsOverviewTimelineChart from "./reports-overview-timeline-chart";

type ReportsOverviewDrawerProps = {
  report: ReportsListItem | null;
  onClose: () => void;
};

export default function ReportsOverviewDrawer({ report, onClose }: ReportsOverviewDrawerProps) {
  const searchParams = useSearchParams();
  const pathname = usePathname();

  const reportHref = (() => {
    if (!report) {
      return "#";
    }

    const query = new URLSearchParams(searchParams.toString());
    const from = pathname === "/account/reports" ? "account-reports" : "reports";
    query.set("from", from);

    const queryString = query.toString();
    const encodedMailId = encodeURIComponent(report.mailId);
    return queryString ? `/report/${encodedMailId}?${queryString}` : `/report/${encodedMailId}`;
  })();

  return (
    <Drawer onClose={onClose} open={report !== null} size="4xl">
      {report ? (
        <>
          <DrawerTitle>Battle overview</DrawerTitle>
          <DrawerDescription>
            A quick glance at the battle report before opening it.
          </DrawerDescription>
          <DrawerBody className="space-y-6">
            <section>
              <Subheading className="mb-3">Battle timeline</Subheading>
              <ReportsOverviewTimelineChart
                startTimestamp={report.timeline.startTimestamp || report.timeStart}
                endTimestamp={report.timeline.endTimestamp || report.timeEnd}
                sampling={report.timeline.sampling}
              />
            </section>
            <section>
              <Subheading className="mb-3">Data summary</Subheading>
              <div className="grid gap-3 lg:grid-cols-2">
                <ReportsOverviewSummaryCard summary={report.summary.sender} title="Sender" />
                <ReportsOverviewSummaryCard summary={report.summary.opponent} title="All Enemies" />
              </div>
            </section>
          </DrawerBody>
          <DrawerActions>
            <Button onClick={onClose} plain>
              Close
            </Button>
            <Button color="zinc" href={reportHref}>
              See full report
            </Button>
          </DrawerActions>
        </>
      ) : null}
    </Drawer>
  );
}
