"use client";

import { useTranslations } from "next-intl";
import { Avatar } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Strong, Text } from "@/components/ui/text";
import { getCommanderName } from "@/hooks/use-commander-name";
import type { DuelCommanderInfo } from "@/lib/types/duel-report";

type DuelCommanderRowProps = {
  commander: DuelCommanderInfo;
  label: string;
};

export function DuelCommanderRow({ commander, label }: DuelCommanderRowProps) {
  const tCommon = useTranslations("common");
  const commanderId = commander.id;
  const commanderName = getCommanderName(
    Number.isFinite(commanderId) ? commanderId : null
  );
  const level = Number.isFinite(commander.level) ? commander.level : null;
  const commanderLabel =
    commanderName ?? commanderId ?? tCommon("labels.unknown");
  const commanderIconSrc = `/lilith/images/commander/${commanderId}.png`;
  const commanderAlt = tCommon("alt.namedIcon", { name: commanderLabel });

  return (
    <Text className="flex flex-wrap items-center gap-2 text-sm">
      <span className="inline-flex items-center gap-1">
        <Avatar
          alt={commanderAlt}
          className="size-12 outline-0!"
          src={commanderIconSrc}
        />
        <Strong>{commanderLabel}</Strong>
      </span>
      <Badge>{label}</Badge>
      {level != null ? (
        <Badge>{tCommon("labels.level", { level })}</Badge>
      ) : null}
    </Text>
  );
}
