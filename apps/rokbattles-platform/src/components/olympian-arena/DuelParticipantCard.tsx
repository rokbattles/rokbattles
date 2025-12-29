import { useId, useState } from "react";
import { DuelCommanderRow } from "@/components/olympian-arena/DuelCommanderRow";
import { Avatar } from "@/components/ui/Avatar";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Subheading } from "@/components/ui/Heading";
import { Text } from "@/components/ui/Text";
import { getArmamentInfo } from "@/hooks/useArmamentName";
import { getInitials, normalizeFrameUrl } from "@/lib/avatar";
import type { DuelBuffEntry, DuelCommanderInfo, DuelParticipantInfo } from "@/lib/types/duelReport";

type NormalizedBuff = {
  id: number;
  value: number;
};

type TroopBuffDisplay = {
  id: number;
  value: number;
  name: string;
  percent: boolean;
};

export function DuelParticipantCard({
  participant,
  isWinner,
}: {
  participant?: DuelParticipantInfo;
  isWinner?: boolean;
}) {
  const [expanded, setExpanded] = useState(false);
  const buffsId = useId();
  const playerName = participant?.player_name?.trim() || "Unknown commander";
  const allianceTag = participant?.alliance?.trim();
  const playerId = participant?.player_id;

  const buffs = normalizeBuffs(participant?.buffs ?? []);
  const displayBuffs = buffs
    .map((buff) => {
      const info = getArmamentInfo(buff.id);
      if (!info?.name) {
        return null;
      }

      return {
        id: buff.id,
        value: buff.value,
        name: info.name,
        percent: info.percent,
      };
    })
    .filter((buff): buff is TroopBuffDisplay => buff != null);
  const visibleBuffs = expanded ? displayBuffs : displayBuffs.slice(0, 10);
  const hasMore = displayBuffs.length > 10;

  const primary = participant?.commanders?.primary;
  const secondary = participant?.commanders?.secondary;
  const showPrimary = hasCommander(primary);
  const showSecondary = hasCommander(secondary);

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
            {isWinner ? <Badge color="emerald">Winner</Badge> : null}
          </div>
          <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-zinc-500 dark:text-zinc-400">
            {typeof playerId === "number" && Number.isFinite(playerId) ? (
              <Badge>ID {playerId}</Badge>
            ) : null}
            {allianceTag ? <Badge>{allianceTag}</Badge> : null}
          </div>
        </div>
      </div>

      <div className="space-y-4">
        {showPrimary || showSecondary ? (
          <div className="space-y-2">
            <Subheading>Commanders</Subheading>
            <div className="space-y-2">
              {showPrimary ? <DuelCommanderRow commander={primary} label="Primary" /> : null}
              {showSecondary ? <DuelCommanderRow commander={secondary} label="Secondary" /> : null}
            </div>
          </div>
        ) : null}
        {displayBuffs.length > 0 ? (
          <div className="space-y-2">
            <Subheading>Troop Buffs</Subheading>
            <div className="space-y-1.5" id={buffsId}>
              {visibleBuffs.map((buff) => (
                <Text key={buff.id} className="flex items-center justify-between">
                  <span>{buff.name}</span>
                  <span className="font-mono text-zinc-900 dark:text-white">
                    {formatBuffValue(buff.value, buff.percent)}
                  </span>
                </Text>
              ))}
            </div>
            {hasMore ? (
              <Button
                plain
                type="button"
                onClick={() => setExpanded((prev) => !prev)}
                aria-expanded={expanded}
                aria-controls={buffsId}
                className="text-sm text-blue-600 data-hover:text-blue-500 dark:text-blue-400 dark:data-hover:text-blue-300"
              >
                {expanded ? "Show less" : "Show more"}
              </Button>
            ) : null}
          </div>
        ) : null}
      </div>
    </div>
  );
}

function hasCommander(commander?: DuelCommanderInfo) {
  const id = commander?.id;
  return typeof id === "number" && Number.isFinite(id);
}

function formatBuffValue(value: number | undefined, isPercent: boolean) {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return isPercent ? "0%" : "+0";
  }

  if (!isPercent) {
    return formatSignedNumber(value);
  }

  const percentValue = Math.abs(value) <= 1 ? value * 100 : value;
  return `${percentValue.toFixed(2)}%`;
}

function formatSignedNumber(value: number) {
  const normalized = Math.round(value * 100) / 100;
  const sign = normalized >= 0 ? "+" : "";
  const formatted = Number.isInteger(normalized)
    ? String(normalized)
    : normalized.toFixed(2).replace(/\.?0+$/, "");
  return `${sign}${formatted}`;
}

function normalizeBuffs(rawBuffs: DuelBuffEntry[]): NormalizedBuff[] {
  const aggregate = new Map<number, number>();

  for (const buff of rawBuffs) {
    const id = typeof buff.id === "number" && Number.isFinite(buff.id) ? buff.id : null;
    if (id == null) {
      continue;
    }

    const value = typeof buff.value === "number" && Number.isFinite(buff.value) ? buff.value : 0;
    aggregate.set(id, (aggregate.get(id) ?? 0) + value);
  }

  return Array.from(aggregate.entries())
    .map(([id, value]) => ({ id, value }))
    .sort((a, b) => a.id - b.id);
}
