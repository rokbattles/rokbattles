"use client";

import { useExtracted } from "next-intl";
import { CommanderLoadoutRow } from "@/components/commander-loadout-row";
import { getFormationName } from "@/hooks/use-formation-name";
import type { RawCommanderInfo } from "@/lib/types/raw-report";

type ReportCommanderRowProps = {
  commander?: RawCommanderInfo;
  formation?: number | null;
};

export function ReportCommanderRow({ commander, formation }: ReportCommanderRowProps) {
  const t = useExtracted();
  const formationName = getFormationName(formation ?? null);
  const formationLabel =
    typeof formation === "number"
      ? (formationName ?? t("Formation {formation}", { formation: formation.toString() }))
      : null;

  return (
    <CommanderLoadoutRow
      id={commander?.id}
      awakened={commander?.awakened}
      level={commander?.level}
      skills={commander?.skills}
      formationLabel={formationLabel}
    />
  );
}
