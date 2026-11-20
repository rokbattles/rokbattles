"use client";

import { commanderMap, getCommanderName as getCommanderNameById } from "@/lib/commander";

export function getCommanderName(id: number | null | undefined) {
  return getCommanderNameById(id);
}

export function useCommanderOptions() {
  const entries = Object.entries(commanderMap).map(([id, commander]) => {
    const name = getCommanderNameById(Number(id)) ?? String(id);

    return {
      id: Number(id),
      name: commander.prime ? `${name} (Prime)` : name,
    };
  });

  return entries.sort((a, b) => a.name.localeCompare(b.name));
}

type TroopType = "infantry" | "cavalry" | "archer" | "leadership" | "integration";

/** Ids of every commander (including primes) that has the given specialty. */
export function commanderIdsWithTalent(talent: TroopType): number[] {
  return Object.entries(commanderMap as Record<string, { talents?: string[] }>)
    .filter(([, commander]) => commander.talents?.includes(talent))
    .map(([id]) => Number(id))
    .sort((a, b) => a - b);
}
