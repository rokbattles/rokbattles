"use client";

import { useExtracted } from "next-intl";
import { ReportArmamentSection } from "@/components/report/report-armament-section";
import { ReportArtifactSection } from "@/components/report/report-artifact-section";
import { ReportCommanderRow } from "@/components/report/report-commander-row";
import { ReportEquipmentSection } from "@/components/report/report-equipment-section";
import { ReportRelicSection } from "@/components/report/report-relic-section";
import { ReportStratagemSection } from "@/components/report/report-stratagem-section";
import { ReportSupportSkills } from "@/components/report/report-support-skills";
import { Badge } from "@/components/ui/badge";
import { Subheading } from "@/components/ui/heading";
import { GameAvatar } from "@/components/v1/game-avatar";
import { GameTranslate } from "@/components/v1/game-translate";
import { getInitials } from "@/lib/avatar";
import { parseArmamentBuffs, parseEquipment, parseSemicolonNumberList } from "@/lib/report/parsers";
import type { RawCommanderInfo, RawParticipantInfo } from "@/lib/types/raw-report";

const ARTIFACT_IDS = new Set([20201, 20202, 20203, 20401, 20402]);

type ReportParticipantCardProps = {
  participant?: RawParticipantInfo;
  showArtifacts?: boolean;
};

export function ReportParticipantCard({
  participant,
  showArtifacts = true,
}: ReportParticipantCardProps) {
  const t = useExtracted();
  const playerName = participant?.player_name?.trim() || t("Unknown commander");
  const allianceTag = participant?.alliance_tag?.trim();
  const playerId = participant?.player_id;

  const equipmentTokens = parseEquipment(participant?.equipment ?? null);
  const artifactTokens = showArtifacts
    ? parseEquipment(participant?.equipment_2 ?? null).filter((token) => ARTIFACT_IDS.has(token.id))
    : [];
  const relics = [
    ...(participant?.primary_commander?.relics ?? []),
    ...(participant?.secondary_commander?.relics ?? []),
  ];
  const inscriptionIds = parseSemicolonNumberList(participant?.inscriptions ?? null);
  const armamentBuffs = parseArmamentBuffs(participant?.armament_buffs ?? null);

  const primaryCommander = participant?.primary_commander;
  const secondaryCommander = participant?.secondary_commander;
  const primaryFormation = participant?.formation;
  const showPrimary = hasCommander(primaryCommander);
  const showSecondary = hasCommander(secondaryCommander);

  return (
    <div className="grid gap-5 lg:row-span-7 lg:grid-rows-subgrid">
      <div className="flex items-start gap-3">
        <GameAvatar
          avatarUrl={participant?.avatar_url ?? null}
          frameUrl={participant?.frame_url ?? null}
          avatarOverride={participant?.avatar_override}
          alt={playerName}
          initials={getInitials(playerName)}
          className="size-12"
        />
        <div className="min-w-0">
          <div className="text-base font-semibold text-zinc-900 dark:text-white">{playerName}</div>
          <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-zinc-500 dark:text-zinc-400">
            {typeof playerId === "number" && Number.isFinite(playerId) ? (
              <Badge>{playerId.toString()}</Badge>
            ) : null}
            {allianceTag ? <Badge>{allianceTag}</Badge> : null}
            {participant?.is_rally ? <Badge>{t("Rally")}</Badge> : null}
          </div>
        </div>
      </div>

      <div className="contents lg:block">
        {showPrimary || showSecondary ? (
          <div className="space-y-2">
            <Subheading>
              <GameTranslate value="LC_COMMON_POWER_DETAILS_COMMANDER_TITLE" />
            </Subheading>
            <div className="space-y-2">
              {showPrimary ? (
                <ReportCommanderRow commander={primaryCommander} formation={primaryFormation} />
              ) : null}
              {showSecondary ? <ReportCommanderRow commander={secondaryCommander} /> : null}
              <ReportSupportSkills
                supportSkills={participant?.support_skills}
                auxiliarySkills={participant?.auxiliary_skills}
              />
            </div>
          </div>
        ) : null}
      </div>
      <div className="contents lg:block">
        <ReportEquipmentSection tokens={equipmentTokens} />
      </div>
      <div className="contents lg:block">
        {showArtifacts ? <ReportArtifactSection tokens={artifactTokens} /> : null}
      </div>
      <div className="contents lg:block">
        <ReportRelicSection relics={relics} />
      </div>
      <div className="contents lg:block">
        <ReportArmamentSection buffs={armamentBuffs} inscriptions={inscriptionIds} />
      </div>
      <div className="contents lg:block">
        <ReportStratagemSection stratagems={participant?.stratagems} />
      </div>
    </div>
  );
}

function hasCommander(commander?: RawCommanderInfo) {
  const id = commander?.id;
  return typeof id === "number" && Number.isFinite(id) && id > 0;
}
