import type { PairingsActivity, PairingsBattleType } from "@/lib/pairings";

export const ACTIVITY_LABELS: Record<PairingsActivity, string> = {
  ark: "Ark of Osiris",
  home: "Home kingdom",
  kvk: "KvK",
  strife: "Supreme Strife",
};

export const BATTLE_LABELS: Record<PairingsBattleType, string> = {
  "open-field": "Open field",
  swarming: "Swarming",
  rally: "Rally",
  garrison: "Garrison",
};

export function readExclusions<T extends string>(
  value: string | null,
  labels: Record<T, string>
): T[] {
  return [...new Set((value ?? "").split(","))].filter((key): key is T =>
    Object.hasOwn(labels, key)
  );
}
