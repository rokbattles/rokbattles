import type {
  BarbarianFortLootDocument,
  BarbarianLootDocument,
  BaulurLootDocument,
} from "@/lib/loot-explorer/api";

export type LootExplorerOption = {
  value: string;
  label: string;
};

export type BarbarianFamily = {
  key: string;
  label: string;
  matches: (item: BarbarianLootDocument) => boolean;
};

export const barbarianFamilies: BarbarianFamily[] = [
  {
    key: "barbarians",
    label: "Barbarians",
    matches: (item) =>
      (item.kind >= 1 && item.kind <= 40) || (item.kind >= 401 && item.kind <= 415),
  },
  {
    key: "barbarian-wolf-tamers-pack-striders",
    label: "Barbarian Wolf Tamers/Pack Striders",
    matches: (item) => item.kind >= 701 && item.kind <= 740,
  },
  {
    key: "barbarian-bone-archers-heavy-archers",
    label: "Barbarian Bone Archers/Heavy Archers",
    matches: (item) => item.kind >= 801 && item.kind <= 840,
  },
  {
    key: "barbarian-beast-riders-blitz-hunters",
    label: "Barbarian Beast Riders/Blitz Hunters",
    matches: (item) => item.kind >= 901 && item.kind <= 940,
  },
  {
    key: "english-soldiers",
    label: "English Soldiers",
    matches: (item) => item.kind >= 150_009 && item.kind <= 150_023,
  },
  {
    key: "marauders",
    label: "Marauders",
    matches: (item) => item.kind === 99 || item.kind === 100,
  },
];

export const fortFamilies = [
  { key: "barbarian-forts", label: "Barbarian Forts", kind: 1 },
  { key: "marauder-encampments", label: "Marauder Encampments", kind: 3 },
  { key: "mottes", label: "Mottes", kind: 4 },
] as const;

export const baulurFamilies = [
  { key: "ironhand-baulur", label: "Ironhand Baulur", kind: 102_000_055 },
  { key: "miser-khaolak", label: "Miser Khaolak", kind: 102_000_063 },
] as const;

export function findBarbarianFamily(
  key: string | undefined,
  items: BarbarianLootDocument[]
): BarbarianFamily {
  const requested = barbarianFamilies.find((family) => family.key === key);
  if (requested && items.some(requested.matches)) {
    return requested;
  }

  return barbarianFamilies.find((family) => items.some(family.matches)) ?? barbarianFamilies[0];
}

export function findFortFamily(key: string | undefined, items: BarbarianFortLootDocument[]) {
  const requested = fortFamilies.find((family) => family.key === key);
  if (requested && items.some((item) => item.kind === requested.kind)) {
    return requested;
  }

  return (
    fortFamilies.find((family) => items.some((item) => item.kind === family.kind)) ??
    fortFamilies[0]
  );
}

export function findBaulurFamily(key: string | undefined, items: BaulurLootDocument[]) {
  const requested = baulurFamilies.find((family) => family.key === key);
  if (requested && items.some((item) => item.kind === requested.kind)) {
    return requested;
  }

  return (
    baulurFamilies.find((family) => items.some((item) => item.kind === family.kind)) ??
    baulurFamilies[0]
  );
}

export function levelOptions(
  items: Array<{ level: number }>,
  formatLabel: (level: number) => string
): LootExplorerOption[] {
  return Array.from(new Set(items.map((item) => item.level)))
    .sort((left, right) => left - right)
    .map((level) => ({ value: String(level), label: formatLabel(level) }));
}
