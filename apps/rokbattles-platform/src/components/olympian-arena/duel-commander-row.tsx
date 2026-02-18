"use client";

import { useExtracted } from "next-intl";
import { Avatar } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Strong, Text } from "@/components/ui/text";
import { getCommanderName } from "@/hooks/use-commander-name";
import type { DuelBattle2Commander } from "@/lib/types/duelbattle2";

type DuelCommanderRowProps = {
  commander: DuelBattle2Commander;
  label: string;
};

export function DuelCommanderRow({ commander, label }: DuelCommanderRowProps) {
  const tCommon = useExtracted();
  const commanderId = commander.id;
  const commanderName = getCommanderName(Number.isFinite(commanderId) ? commanderId : null);
  const level = Number.isFinite(commander.level) ? commander.level : null;
  const commanderLabel = commanderName ?? commanderId ?? tCommon("Unknown");
  const commanderIconSrc = `https://cdn.rokbattles.com/game/commander/${commanderId}.png`;
  const commanderAlt = tCommon("{name} icon", { name: commanderLabel.toString() });

  return (
    <Text className="flex flex-wrap items-center gap-2 text-sm">
      <span className="inline-flex items-center gap-1">
        <Avatar src={commanderIconSrc} alt={commanderAlt} className="size-12 outline-0!" />
        <Strong>{commanderLabel}</Strong>
      </span>
      <Badge>{label}</Badge>
      {level != null ? <Badge>{tCommon("Lvl {level}", { level: level.toString() })}</Badge> : null}
    </Text>
  );
}
