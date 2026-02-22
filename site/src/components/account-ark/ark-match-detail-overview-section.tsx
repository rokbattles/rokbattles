import { getExtracted } from "next-intl/server";
import {
  DescriptionDetails,
  DescriptionList,
  DescriptionTerm,
} from "@/components/ui/description-list";
import { Subheading } from "@/components/ui/heading";
import { formatArkMetricValue } from "@/lib/ark/detail-format";
import type { ArkMatchDetailOverview } from "@/lib/types/ark";

type ArkMatchDetailOverviewSectionProps = {
  overview: ArkMatchDetailOverview;
};

export async function ArkMatchDetailOverviewSection({
  overview,
}: ArkMatchDetailOverviewSectionProps) {
  const t = await getExtracted();
  const unavailableLabel = t("N/A");

  return (
    <div className="space-y-2">
      <Subheading>{t("Overview")}</Subheading>
      <DescriptionList>
        <DescriptionTerm>{t("Rank")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(overview.rank, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Score")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(overview.score, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Total Battles")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(overview.battles, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Kill Points Gain")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(overview.killPointsGain, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Kill Points Loss")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(overview.killPointsLoss, unavailableLabel)}
        </DescriptionDetails>
      </DescriptionList>
    </div>
  );
}
