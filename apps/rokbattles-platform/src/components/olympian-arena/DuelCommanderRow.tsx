import { Avatar } from "@/components/ui/Avatar";
import { Badge } from "@/components/ui/Badge";
import { Strong, Text } from "@/components/ui/Text";
import { getCommanderName } from "@/hooks/useCommanderName";
import type { DuelCommanderInfo } from "@/lib/types/duelReport";

type DuelCommanderRowProps = {
  commander?: DuelCommanderInfo;
  label: "Primary" | "Secondary";
};

export function DuelCommanderRow({ commander, label }: DuelCommanderRowProps) {
  const commanderId = commander?.id;
  const commanderName = getCommanderName(commanderId ?? null);
  const level = typeof commander?.level === "number" ? commander.level : null;
  const commanderLabel = commanderName ?? commanderId ?? "Unknown";
  const commanderIconSrc = `/lilith/images/commander/${commanderId}.png`;

  return (
    <Text className="flex flex-wrap items-center gap-2 text-sm">
      <span className="inline-flex items-center gap-1">
        <Avatar
          src={commanderIconSrc}
          alt={`${commanderLabel} icon`}
          className="size-12 outline-0!"
        />
        <Strong>{commanderLabel}</Strong>
      </span>
      <Badge>{label}</Badge>
      {level != null ? <Badge>Lvl {level}</Badge> : null}
    </Text>
  );
}
