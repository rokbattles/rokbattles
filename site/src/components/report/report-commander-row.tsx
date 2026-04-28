"use client";

import { useExtracted } from "next-intl";
import { CommanderIcon } from "@/components/commander-icon";
import { Badge } from "@/components/ui/badge";
import { Strong, Text } from "@/components/ui/text";
import { getCommanderName } from "@/hooks/use-commander-name";
import { getFormationName } from "@/hooks/use-formation-name";
import type { RawCommanderInfo } from "@/lib/types/raw-report";

type ReportCommanderRowProps = {
  commander?: RawCommanderInfo;
  formation?: number | null;
};

export function ReportCommanderRow({ commander, formation }: ReportCommanderRowProps) {
  const t = useExtracted();
  const commanderId = commander?.id;
  const commanderName = getCommanderName(commanderId ?? null);
  const formationName = getFormationName(formation ?? null);
  const level = typeof commander?.level === "number" ? commander.level : null;
  const commanderLabel = commanderName ?? commanderId ?? t("Unknown");
  const commanderAlt = t("{name} icon", { name: commanderLabel.toString() });
  const formationLabel =
    typeof formation === "number"
      ? (formationName ?? t("Formation {formation}", { formation: formation.toString() }))
      : null;

  return (
    <Text className="flex flex-wrap items-center gap-2 text-sm">
      <span className="inline-flex items-center gap-1">
        <CommanderIcon
          alt={commanderAlt}
          awakened={commander?.awakened}
          className="size-12 outline-0!"
          id={commanderId}
          sizes="48px"
        />
        <Strong>{commanderLabel}</Strong>
      </span>
      {formationLabel ? <Badge>{formationLabel}</Badge> : null}
      {level != null ? <Badge>{t("Lvl {level}", { level: level.toString() })}</Badge> : null}
    </Text>
  );
}
