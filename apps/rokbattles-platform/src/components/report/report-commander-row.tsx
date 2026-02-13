"use client";

import { useTranslations } from "next-intl";
import { Avatar } from "@/components/ui/avatar";
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
  const t = useTranslations("report");
  const tCommon = useTranslations("common");
  const commanderId = commander?.id;
  const commanderName = getCommanderName(commanderId ?? null);
  const formationName = getFormationName(formation ?? null);
  const level = typeof commander?.level === "number" ? commander.level : null;
  const skillSummary = commander?.skills?.trim();
  const commanderLabel = commanderName ?? commanderId ?? tCommon("labels.unknown");
  const commanderIconSrc = `/lilith/images/commander/${commanderId}.png`;
  const commanderAlt = tCommon("alt.namedIcon", { name: commanderLabel });
  const formationLabel =
    typeof formation === "number"
      ? (formationName ?? t("commanders.formationFallback", { formation }))
      : null;

  return (
    <Text className="flex flex-wrap items-center gap-2 text-sm">
      <span className="inline-flex items-center gap-1">
        <Avatar src={commanderIconSrc} alt={commanderAlt} className="size-12 outline-0!" />
        <Strong>{commanderLabel}</Strong>
      </span>
      {formationLabel ? <Badge>{formationLabel}</Badge> : null}
      {level != null ? <Badge>{tCommon("labels.level", { level })}</Badge> : null}
      {skillSummary ? <Badge>{skillSummary}</Badge> : null}
    </Text>
  );
}
