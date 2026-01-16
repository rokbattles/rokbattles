export type EquipmentToken = {
  slot: number;
  id: number;
  craft?: number;
  attr?: number;
};

export function parseEquipment(
  raw: string | null | undefined
): EquipmentToken[] {
  if (!raw) {
    return [];
  }

  const trimmed = raw.trim();
  if (!trimmed) {
    return [];
  }

  const normalized = trimmed.replace(/^[{\s]+|[}\s]+$/g, "");
  if (!normalized) {
    return [];
  }

  const tokens = normalized
    .split(",")
    .map((token) => token.trim())
    .filter(Boolean);

  const result: EquipmentToken[] = [];

  for (const token of tokens) {
    const [slotStr, idCraft = "", attrStr] = token.split(":");
    const slot = Number(slotStr);
    if (!Number.isFinite(slot)) {
      continue;
    }

    const [idStr, craftStr] = idCraft.split("_");
    const id = Number(idStr);
    if (!Number.isFinite(id)) {
      continue;
    }

    const craft = craftStr ? Number(craftStr) : undefined;
    const attr = attrStr !== undefined ? Number(attrStr) : undefined;

    result.push({
      slot,
      id,
      craft: Number.isFinite(craft) ? craft : undefined,
      attr: Number.isFinite(attr) ? attr : undefined,
    });
  }

  return result.sort((a, b) => a.slot - b.slot);
}

export function parseSemicolonNumberList(
  raw: string | null | undefined
): number[] {
  if (!raw) {
    return [];
  }

  return raw
    .split(";")
    .map((value) => Number(value))
    .filter((value) => Number.isFinite(value) && value !== -1);
}

export type ArmamentBuff = {
  id: number;
  value: number;
};

export function parseArmamentBuffs(
  raw: string | null | undefined
): ArmamentBuff[] {
  if (!raw) {
    return [];
  }

  const tokens = raw
    .split(";")
    .map((token) => token.trim())
    .filter(Boolean);

  const aggregate = new Map<number, number>();

  for (const token of tokens) {
    const [idStr, valueStr] = token.split("_");
    const id = Number(idStr);
    if (!Number.isFinite(id)) {
      continue;
    }

    const value = Number(valueStr);
    const numericValue = Number.isFinite(value) ? value : 0;
    aggregate.set(id, (aggregate.get(id) ?? 0) + numericValue);
  }

  return Array.from(aggregate.entries())
    .map(([id, value]) => ({
      id,
      value,
    }))
    .sort((a, b) => a.id - b.id);
}

export type InscriptionRarity = "common" | "rare" | "special";

export function getInscriptionRarity(id: number): InscriptionRarity {
  if (!Number.isFinite(id)) {
    return "common";
  }

  if (id >= 1000) {
    const lastDigit = id % 10;
    if (lastDigit === 1) {
      return "special";
    }
    if (lastDigit === 2) {
      return "rare";
    }
  }

  return "common";
}
