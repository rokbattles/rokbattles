"use client";

import { useExtracted } from "next-intl";
import {
  DescriptionDetails,
  DescriptionList,
  DescriptionTerm,
} from "@/components/ui/description-list";
import { Subheading } from "@/components/ui/heading";
import { GameTranslate } from "@/components/v1/game-translate";
import { formatArkMetricValue, formatArkPercentValue } from "@/lib/ark/detail-format";
import type { ArkMatchDetailIndividualResults } from "@/lib/types/ark";

type ArkMatchDetailIndividualResultsSectionProps = {
  individualResults: ArkMatchDetailIndividualResults;
};

export function ArkMatchDetailIndividualResultsSection({
  individualResults,
}: ArkMatchDetailIndividualResultsSectionProps) {
  const t = useExtracted();
  const unavailableLabel = t("N/A");

  return (
    <div className="space-y-2">
      <Subheading>{t("Individual Results")}</Subheading>
      <DescriptionList>
        <DescriptionTerm>
          <GameTranslate value="LC_BATTLEFIELD_RESULT_FLAG" />
        </DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.arkOfOsirisScore, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>
          <GameTranslate value="LC_BATTLEFIELD_RESULT_KILL" />
        </DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.killScore, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>
          <GameTranslate value="LC_BATTLEFIELD_RESULT_BUILDING" />
        </DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.occupationScore, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>
          <GameTranslate value="LC_BATTLEFIELD_RESULT_RESOURCE" />
        </DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.provisionsScore, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>
          <GameTranslate value="LC_BATTLEFIELD_STAT_DATA3_02" />
        </DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.battlesWin, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Battles Lost")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.battlesLose, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>
          <GameTranslate value="LC_BATTLEFIELD_RESULT_WIN" />
        </DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkPercentValue(individualResults.winRate, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>
          <GameTranslate value="LC_BATTLEFIELD_RESULT_KILL_NUM" />
        </DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.kills, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>
          <GameTranslate value="LC_COMMON_SEVERELY_WOUNDED" />
        </DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.severelyWounded, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>
          <GameTranslate value="LC_BATTLEFIELD_RESULT_HEAL" />
        </DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.unitsHealed, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>{t("Speedups (minutes)")}</DescriptionTerm>
        <DescriptionDetails className="tabular-nums">
          {formatArkMetricValue(individualResults.speedups, unavailableLabel)}
        </DescriptionDetails>
        <DescriptionTerm>
          <GameTranslate value="LC_BATTLEFIELD_END_TELEPORT" />
        </DescriptionTerm>
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
