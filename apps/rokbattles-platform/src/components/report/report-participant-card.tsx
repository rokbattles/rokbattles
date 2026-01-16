"use client";

import { useTranslations } from "next-intl";
import { ReportArmamentSection } from "@/components/report/report-armament-section";
import { ReportArtifactSection } from "@/components/report/report-artifact-section";
import { ReportCommanderRow } from "@/components/report/report-commander-row";
import { ReportEquipmentSection } from "@/components/report/report-equipment-section";
import { Avatar } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Subheading } from "@/components/ui/heading";
import { getInitials, normalizeFrameUrl } from "@/lib/avatar";
import { parseArmamentBuffs, parseEquipment, parseSemicolonNumberList } from "@/lib/report/parsers";
import type { RawCommanderInfo, RawParticipantInfo } from "@/lib/types/raw-report";

const ARTIFACT_IDS = new Set([20401, 20402]);

type ReportParticipantCardProps = {
  participant?: RawParticipantInfo;
  showArtifacts?: boolean;
};

export function ReportParticipantCard({
  participant,
  showArtifacts = true,
}: ReportParticipantCardProps) {
  const tCommon = useTranslations("common");
  const playerName = participant?.player_name?.trim() || tCommon("labels.unknownCommander");
  const allianceTag = participant?.alliance_tag?.trim();
  const playerId = participant?.player_id;

  const equipmentTokens = parseEquipment(participant?.equipment ?? null);
  const artifactTokens = showArtifacts
    ? parseEquipment(participant?.equipment_2 ?? null).filter((token) => ARTIFACT_IDS.has(token.id))
    : [];
  const inscriptionIds = parseSemicolonNumberList(participant?.inscriptions ?? null);
  const armamentBuffs = parseArmamentBuffs(participant?.armament_buffs ?? null);

  const primaryCommander = participant?.primary_commander;
  const secondaryCommander = participant?.secondary_commander;
  const primaryFormation = participant?.formation;
  const showPrimary = hasCommander(primaryCommander);
  const showSecondary = hasCommander(secondaryCommander);

  return (
    <div className="flex flex-col gap-5">
      <div className="flex items-start gap-3">
        <Avatar
          src={participant?.avatar_url ?? undefined}
          frameSrc={normalizeFrameUrl(participant?.frame_url)}
          alt={playerName}
          initials={getInitials(playerName)}
          className="size-12"
        />
        <div className="min-w-0">
          <div className="text-base font-semibold text-zinc-900 dark:text-white">{playerName}</div>
          <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-zinc-500 dark:text-zinc-400">
            {typeof playerId === "number" && Number.isFinite(playerId) ? (
              <Badge>{tCommon("labels.id", { id: playerId })}</Badge>
            ) : null}
            {allianceTag ? <Badge>{allianceTag}</Badge> : null}
            {participant?.is_rally ? <Badge>{tCommon("labels.rally")}</Badge> : null}
          </div>
        </div>
      </div>

      <div className="space-y-4">
        {showPrimary || showSecondary ? (
          <div className="space-y-2">
            <Subheading>{tCommon("labels.commanders")}</Subheading>
            <div className="space-y-2">
              {showPrimary ? (
                <ReportCommanderRow commander={primaryCommander} formation={primaryFormation} />
              ) : null}
              {showSecondary ? <ReportCommanderRow commander={secondaryCommander} /> : null}
            </div>
          </div>
        ) : null}
        <ReportEquipmentSection tokens={equipmentTokens} />
        {showArtifacts ? <ReportArtifactSection tokens={artifactTokens} /> : null}
        <ReportArmamentSection buffs={armamentBuffs} inscriptions={inscriptionIds} />
      </div>
    </div>
  );
}

function hasCommander(commander?: RawCommanderInfo) {
  const id = commander?.id;
  return typeof id === "number" && Number.isFinite(id);
}
