import "server-only";

import { normalizeTimestampMillis } from "@/lib/datetime";

export function parseNumeric(value: unknown): number | null {
  if (value == null) {
    return null;
  }

  const parsed = typeof value === "bigint" ? Number(value) : Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

export function extractEventTimeMillis(mailTime: unknown): number | null {
  return normalizeTimestampMillis(mailTime);
}

export function isBarbarian(npcType: number | null, npcBType: number | null): boolean {
  if (npcType == null || npcBType == null || npcBType !== 1) {
    return false;
  }

  const isHomeBarbarian = npcType >= 1 && npcType <= 40;
  const isKvkBarbarian = npcType >= 401 && npcType <= 415;
  return isHomeBarbarian || isKvkBarbarian;
}

export function toDateKey(eventTimeMillis: number): string {
  return new Date(eventTimeMillis).toISOString().slice(0, 10);
}
