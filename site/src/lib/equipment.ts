export type EquipmentTroopType =
  | "infantry"
  | "archer"
  | "cavalry"
  | "integration"
  | "leadership"
  | "engineering";

const TROOP_TYPE_ICON_PATHS: Record<EquipmentTroopType, string> = {
  infantry: "btn_BlackSmithsShopSystemInfantry.Sprite.-8042821011777516562__781710f5d2f4.png",
  archer: "btn_BlackSmithsShopSystemArcher.Sprite.-8821678283852625625__0349d07a5d23.png",
  cavalry: "btn_BlackSmithsShopSystemCavalry.Sprite.-1545899140773578379__69ba8909171f.png",
  integration: "btn_BlackSmithsShopSystemIntegration.Sprite.6769590931564808732__8f64da276457.png",
  leadership: "btn_BlackSmithsShopSystemLeadership.Sprite.-6704481556664019519__47b6bcae4b07.png",
  engineering: "btn_BlackSmithsShopSystemVehicle.Sprite.-416128535698636701__ff4f7019fa82.png",
};

const ATTR_TROOP_TYPE_MAP: Record<number, EquipmentTroopType> = {
  1: "infantry",
  2: "archer",
  3: "cavalry",
  4: "integration",
  5: "leadership",
  16: "engineering",
};

export function getEquipmentTierInfo(attr?: number) {
  if (typeof attr !== "number" || !Number.isFinite(attr)) {
    return { tier: undefined, isSpecialTalent: false, troopType: undefined };
  }

  const numeric = Number(attr);
  const troopTypeCode = Math.floor(numeric / 10);
  const troopType = ATTR_TROOP_TYPE_MAP[troopTypeCode];
  const isSpecialTalent = troopType != null;
  const base = isSpecialTalent ? numeric % 10 : numeric;
  const tier = Number.isFinite(base) ? base : undefined;

  return { tier, isSpecialTalent, troopType };
}

export function getEquipmentTroopTypeIconSrc(troopType?: EquipmentTroopType) {
  if (!troopType) {
    return undefined;
  }

  return `https://cdn.rokbattles.com/game/sprites/${TROOP_TYPE_ICON_PATHS[troopType]}`;
}

export function toRomanNumeral(value: number | undefined) {
  if (typeof value !== "number") {
    return null;
  }

  const numerals = ["", "I", "II", "III", "IV", "V"];
  return numerals[value] ?? null;
}
