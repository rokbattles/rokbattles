"use client";

import { useExtracted } from "next-intl";
import { Button } from "@/components/ui/button";
import {
  Drawer,
  DrawerActions,
  DrawerBody,
  DrawerDescription,
  DrawerTitle,
} from "@/components/ui/drawer";
import type { ExploreBattleReportRow } from "@/lib/types/explore-battle-reports";

export default function ExploreReportOverviewDrawer({
  report,
  onClose,
}: {
  report: ExploreBattleReportRow | null;
  onClose: () => void;
}) {
  const t = useExtracted();

  return (
    <Drawer onClose={onClose} open={report !== null} size="lg">
      {report ? (
        <>
          <DrawerTitle>{t("Battle overview")}</DrawerTitle>
          <DrawerDescription>
            {t("Quick glance at this report before opening the full battle.")}
          </DrawerDescription>
          <DrawerBody>{t("ROK Battles")}</DrawerBody>
          <DrawerActions>
            <Button onClick={onClose} plain>
              Close
            </Button>
            <Button
              color="dark/zinc"
              href={
                report
                  ? `/report/${encodeURIComponent(report.mailId)}`
                  : undefined
              }
            >
              See full battle
            </Button>
          </DrawerActions>
        </>
      ) : null}
    </Drawer>
  );
}
