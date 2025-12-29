import { ReportArmamentSection } from "@/components/report/ReportArmamentSection";
import { ReportArtifactSection } from "@/components/report/ReportArtifactSection";
import { ReportCommanderRow } from "@/components/report/ReportCommanderRow";
import { ReportEquipmentSection } from "@/components/report/ReportEquipmentSection";
import { Avatar } from "@/components/ui/Avatar";
import { Badge } from "@/components/ui/Badge";
import { Subheading } from "@/components/ui/Heading";
import { getInitials, normalizeFrameUrl } from "@/lib/avatar";
import { parseArmamentBuffs, parseEquipment, parseSemicolonNumberList } from "@/lib/report/parsers";
import type { RawCommanderInfo, RawParticipantInfo } from "@/lib/types/rawReport";

type ParticipantSide = "self" | "enemy";

const ARTIFACT_IDS = new Set([20401, 20402]);

export function ReportParticipantCard({
  participant,
  side,
}: {
  participant?: RawParticipantInfo;
  side: ParticipantSide;
}) {
  const playerName = participant?.player_name?.trim() || "Unknown commander";
  const allianceTag = participant?.alliance_tag?.trim();
  const playerId = participant?.player_id;

  const equipmentTokens = parseEquipment(participant?.equipment ?? null);
  const artifactTokens = parseEquipment(participant?.equipment_2 ?? null).filter((token) =>
    ARTIFACT_IDS.has(token.id)
  );
  const inscriptionIds = parseSemicolonNumberList(participant?.inscriptions ?? null);
  const armamentBuffs = parseArmamentBuffs(participant?.armament_buffs ?? null);

  const sideBadgeColor = side === "self" ? "blue" : "rose";
  const sideBadgeLabel = side === "self" ? "Self" : "Enemy";

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
          <div className="flex items-center gap-2">
            <div className="text-base font-semibold text-zinc-900 dark:text-white">
              {playerName}
            </div>
            <Badge color={sideBadgeColor}>{sideBadgeLabel}</Badge>
          </div>
          <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-zinc-500 dark:text-zinc-400">
            {typeof playerId === "number" && Number.isFinite(playerId) ? (
              <Badge>ID {playerId}</Badge>
            ) : null}
            {allianceTag ? <Badge>{allianceTag}</Badge> : null}
            {participant?.is_rally ? <Badge>Rally</Badge> : null}
          </div>
        </div>
      </div>

      <div className="space-y-4">
        {showPrimary || showSecondary ? (
          <div className="space-y-2">
            <Subheading>Commanders</Subheading>
            <div className="space-y-2">
              {showPrimary ? (
                <ReportCommanderRow commander={primaryCommander} formation={primaryFormation} />
              ) : null}
              {showSecondary ? <ReportCommanderRow commander={secondaryCommander} /> : null}
            </div>
          </div>
        ) : null}
        <ReportEquipmentSection tokens={equipmentTokens} />
        <ReportArtifactSection tokens={artifactTokens} />
        <ReportArmamentSection buffs={armamentBuffs} inscriptions={inscriptionIds} />
      </div>
    </div>
  );
}

function hasCommander(commander?: RawCommanderInfo) {
  const id = commander?.id;
  return typeof id === "number" && Number.isFinite(id);
}
