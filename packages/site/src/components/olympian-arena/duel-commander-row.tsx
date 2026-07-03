"use client";

import { useExtracted } from "next-intl";
import { CommanderIcon } from "@/components/commander-icon";
import { Badge } from "@/components/ui/badge";
import { Strong, Text } from "@/components/ui/text";
import { getCommanderName } from "@/hooks/use-commander-name";
import type { DuelBattle2Commander } from "@/lib/types/duelbattle2";

type DuelCommanderRowProps = {
  commander: DuelBattle2Commander;
};

export function DuelCommanderRow({ commander }: DuelCommanderRowProps) {
  const t = useExtracted();
  const commanderId = commander.id;
  const commanderName = getCommanderName(Number.isFinite(commanderId) ? commanderId : null);
  const level = Number.isFinite(commander.level) ? commander.level : null;
  const commanderLabel = commanderName ?? commanderId ?? t("Unknown");
  const commanderAlt = t("{name} icon", { name: commanderLabel.toString() });

  return (
    <Text className="flex flex-wrap items-center gap-2 text-sm">
      <span className="inline-flex items-center gap-1">
        <CommanderIcon
          alt={commanderAlt}
          awakened={commander.awakened}
          className="size-12 outline-0!"
          id={commanderId}
          sizes="48px"
        />
        <Strong>{commanderLabel}</Strong>
      </span>
      {level != null ? <Badge>{t("Lvl {level}", { level: level.toString() })}</Badge> : null}
    </Text>
  );
}
