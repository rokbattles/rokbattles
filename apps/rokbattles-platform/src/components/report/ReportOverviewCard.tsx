import { Subheading } from "@/components/ui/Heading";
import type { RawOverview, RawParticipantInfo } from "@/lib/types/rawReport";
import { ReportOverviewColumn } from "@/components/report/ReportOverviewColumn";

export function ReportOverviewCard({
  overview,
  selfParticipant,
  enemyParticipant,
}: {
  overview: RawOverview;
  selfParticipant?: RawParticipantInfo;
  enemyParticipant?: RawParticipantInfo;
}) {
  const formatter = new Intl.NumberFormat("en-US", {
    maximumFractionDigits: 0,
  });

  return (
    <div className="space-y-4">
      <Subheading>Data summary</Subheading>
      <div className="grid gap-4 sm:gap-6 md:grid-cols-2">
        <ReportOverviewColumn
          side="self"
          overview={overview}
          participant={selfParticipant}
          formatter={formatter}
        />
        <ReportOverviewColumn
          side="enemy"
          overview={overview}
          participant={enemyParticipant}
          formatter={formatter}
        />
      </div>
    </div>
  );
}
