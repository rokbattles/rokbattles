import { getExtracted } from "next-intl/server";
import {
  DescriptionDetails,
  DescriptionList,
  DescriptionTerm,
} from "@/components/ui/description-list";
import { Subheading } from "@/components/ui/heading";
import { formatArkMetricValue, formatArkPercentValue } from "@/lib/ark/detail-format";
import type { ArkMatchDetailIndividualResults } from "@/lib/types/ark";

type ArkMatchDetailIndividualResultsSectionProps = {
  individualResults: ArkMatchDetailIndividualResults;
};

export async function ArkMatchDetailIndividualResultsSection({
  individualResults,
}: ArkMatchDetailIndividualResultsSectionProps) {
  const t = await getExtracted();
  const unavailableLabel = t("N/A");

  return (
    <div className="space-y-2">
      <Subheading>{t("Individual Results")}</Subheading>
      <DescriptionList>
        <DescriptionTerm>{t("Ark of Osiris Score")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.arkOfOsirisScore, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Kill Score")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.killScore, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Occupation Score")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.occupationScore, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Provisions Score")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.provisionsScore, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Battles Won")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.battlesWin, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Battles Lost")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.battlesLose, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Win Percentage")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkPercentValue(individualResults.winRate, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Kills")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.kills, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Severely Wounded")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.severelyWounded, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Units Healed")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.unitsHealed, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Speedups (minutes)")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.speedups, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Teleports")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.teleports, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Structures Entered")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.structures, unavailableLabel)}
        </DescriptionDetails>
      </DescriptionList>
    </div>
  );
}
