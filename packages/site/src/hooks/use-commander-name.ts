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
