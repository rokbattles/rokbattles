"use client";

import { CommanderLoadoutRow } from "@/components/commander-loadout-row";
import type { RawCommanderInfo } from "@/lib/types/raw-report";

type ReportCommanderRowProps = {
  commander?: RawCommanderInfo;
};

export function ReportCommanderRow({ commander }: ReportCommanderRowProps) {
  return (
    <CommanderLoadoutRow
      id={commander?.id}
      awakened={commander?.awakened}
      skinId={commander?.skins?.["22"]}
      level={commander?.level}
      skills={commander?.skills}
    />
  );
}
