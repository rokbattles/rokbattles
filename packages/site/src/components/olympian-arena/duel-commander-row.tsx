"use client";

import { CommanderLoadoutRow } from "@/components/commander-loadout-row";
import type { DuelBattle2Commander } from "@/lib/types/duelbattle2";

type DuelCommanderRowProps = {
  commander: DuelBattle2Commander;
};

export function DuelCommanderRow({ commander }: DuelCommanderRowProps) {
  return (
    <CommanderLoadoutRow
      id={commander.id}
      awakened={commander.awakened}
      level={commander.level}
      skills={commander.skills}
    />
  );
}
