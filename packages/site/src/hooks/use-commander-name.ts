"use client";

import { resolveLocale } from "@/i18n/locale";
import { commanderMap, getCommanderName as getCommanderNameByLocale } from "@/lib/commander";

export function getCommanderName(id: number | null | undefined, locale?: string) {
  const requestedLocale = resolveLocale(locale);
  return getCommanderNameByLocale(id, requestedLocale);
}

export function useCommanderOptions(locale?: string) {
  const requestedLocale = resolveLocale(locale);
  const entries = Object.entries(commanderMap).map(([id, commander]) => {
    const localizedName = getCommanderNameByLocale(Number(id), requestedLocale) ?? String(id);

    return {
      id: Number(id),
      name: commander.prime ? `${localizedName} (Prime)` : localizedName,
    };
  });

  return entries.sort((a, b) => a.name.localeCompare(b.name));
}
