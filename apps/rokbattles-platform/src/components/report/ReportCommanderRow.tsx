import { Avatar } from "@/components/ui/Avatar";
import { Badge } from "@/components/ui/Badge";
import { Strong, Text } from "@/components/ui/Text";
import { getCommanderName } from "@/hooks/useCommanderName";
import { getFormationName } from "@/hooks/useFormationName";
import type { RawCommanderInfo } from "@/lib/types/rawReport";

type ReportCommanderRowProps = {
  commander?: RawCommanderInfo;
  formation?: number | null;
};

export function ReportCommanderRow({ commander, formation }: ReportCommanderRowProps) {
  const commanderId = commander?.id;
  const commanderName = getCommanderName(commanderId ?? null);
  const formationName = getFormationName(formation ?? null);
  const level = typeof commander?.level === "number" ? commander.level : null;
  const skillSummary = commander?.skills?.trim();
  const commanderLabel = commanderName ?? commanderId ?? "Unknown";
  const commanderIconSrc = `/lilith/images/commander/${commanderId}.png`;

  return (
    <Text className="flex flex-wrap items-center gap-2 text-sm">
      <span className="inline-flex items-center gap-1">
        <Avatar src={commanderIconSrc} alt={`${commanderLabel} icon`} className="size-12 outline-0!" />
        <Strong>{commanderLabel}</Strong>
      </span>
      {typeof formation === "number" ? (
        <Badge>{formationName ?? `Formation ${formation}`}</Badge>
      ) : null}
      {level != null ? <Badge>Lvl {level}</Badge> : null}
      {skillSummary ? <Badge>{skillSummary}</Badge> : null}
    </Text>
  );
}
