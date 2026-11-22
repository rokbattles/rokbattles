import Image from "next/image";
import type { CSSProperties } from "react";

import { getEquipmentName, getEquipmentSlotName } from "@/hooks/use-equipment-name";
import { getCommanderTalents } from "@/lib/commander";
import {
  AWAKEN_FRAME_SPRITE_URL,
  AWAKEN_NAMEPLATE_SPRITE_URL,
  EQUIPMENT_QUALITY_GLOW_SPRITE_URL,
  getEquipmentQualityColor,
  getEquipmentRarity,
  getEquipmentSlotSpriteUrl,
  getEquipmentSpriteUrl,
  getEquipmentTalentSpriteUrl,
  getEquipmentTierInfo,
  getIconicTierSpriteUrl,
  toRomanNumeral,
} from "@/lib/equipment";
import type { EquipmentToken } from "@/lib/report/parsers";

type SlotPosition = {
  slot: number;
  x: number;
  y: number;
};

// Slot centers in draw order, measured in hundredths of a slot. Positive Y points up.
const SLOT_POSITIONS: readonly SlotPosition[] = [
  { slot: 2, x: 1.7, y: 220.9 },
  { slot: 3, x: -0.9, y: 80.4 },
  { slot: 1, x: -75.7, y: 5.6 },
  { slot: 4, x: 75.9, y: 5.6 },
  { slot: 5, x: -0.9, y: -70.2 },
  { slot: 7, x: -75.7, y: -144.2 },
  { slot: 8, x: 75.9, y: -144.2 },
  { slot: 6, x: -0.9, y: -220.9 },
];

const SLOT_CLASS =
  "absolute block size-(--gear-slot) -translate-x-1/2 -translate-y-1/2 select-none";

type GearSlotProps = {
  position: SlotPosition;
  token?: EquipmentToken;
  commanderTalents?: readonly string[];
};

function GearSlot({ position, token, commanderTalents }: GearSlotProps) {
  const { slot, x, y } = position;
  const anchor = {
    left: `calc(50% + var(--gear-slot) * ${x / 100})`,
    top: `calc(50% - var(--gear-slot) * ${y / 100})`,
  };
  const spriteUrl = getEquipmentSpriteUrl(token?.id);
  const name = token ? (getEquipmentName(token.id) ?? String(token.id)) : undefined;

  if (!spriteUrl) {
    return <EmptyGearSlot slot={slot} anchor={anchor} name={name} />;
  }

  const rarity = getEquipmentRarity(token?.id);
  const qualityColor = getEquipmentQualityColor(rarity);
  const { tier, troopType } = getEquipmentTierInfo(token?.attr);
  const iconicSrc = rarity === "legendary" ? getIconicTierSpriteUrl(tier) : undefined;
  const talentSrc = getEquipmentTalentSpriteUrl(troopType);
  const talentInactive = troopType && commanderTalents?.includes(troopType) === false;

  const label = [
    name,
    iconicSrc ? `Iconic ${toRomanNumeral(tier)}` : undefined,
    troopType ? `${troopType} talent${talentInactive ? " (inactive)" : ""}` : undefined,
  ]
    .filter(Boolean)
    .join(" – ");

  return (
    <span className={SLOT_CLASS} style={anchor} role="img" aria-label={label} title={label}>
      <Image
        src={spriteUrl}
        alt=""
        width={64}
        height={64}
        className="absolute inset-[15%] size-[70%] object-contain"
        unoptimized
        draggable={false}
      />
      {qualityColor ? (
        <span
          className="pointer-events-none absolute -inset-[19%] mask-contain mask-center mask-no-repeat"
          style={{
            backgroundColor: qualityColor,
            maskImage: `url("${EQUIPMENT_QUALITY_GLOW_SPRITE_URL}")`,
          }}
        />
      ) : null}
      {iconicSrc ? <AwakeningDecoration iconicSrc={iconicSrc} /> : null}
      {talentSrc ? (
        <Image
          src={talentSrc}
          alt=""
          width={30}
          height={30}
          className={`pointer-events-none absolute top-[74.9%] left-[70.3%] w-[30%] -translate-x-1/2 -translate-y-1/2 object-contain ${talentInactive ? "grayscale" : ""}`}
          unoptimized
          draggable={false}
        />
      ) : null}
    </span>
  );
}

function EmptyGearSlot({
  slot,
  anchor,
  name,
}: {
  slot: number;
  anchor: CSSProperties;
  name?: string;
}) {
  const slotName = getEquipmentSlotName(slot);
  const placeholderSrc = getEquipmentSlotSpriteUrl(slot);
  const label = name ? `${slotName}: ${name}` : `Empty ${slotName.toLowerCase()} slot`;

  // A 97.5% square rotated 45 degrees matches the width of the quality glow.
  return (
    <span className={SLOT_CLASS} style={anchor} role="img" aria-label={label} title={label}>
      <span className="pointer-events-none absolute inset-[1.25%] -rotate-45 rounded-[9%] border border-white/10" />
      {placeholderSrc ? (
        <Image
          src={placeholderSrc}
          alt=""
          width={60}
          height={60}
          className="absolute inset-[22.5%] size-[55%] object-contain opacity-[.13]"
          unoptimized
          draggable={false}
        />
      ) : null}
    </span>
  );
}

function AwakeningDecoration({ iconicSrc }: { iconicSrc: string }) {
  return (
    <>
      <Image
        src={AWAKEN_FRAME_SPRITE_URL}
        alt=""
        width={132}
        height={132}
        className="pointer-events-none absolute top-1/2 left-1/2 size-[108.24%] max-w-none -translate-x-1/2 -translate-y-1/2 -rotate-45 object-contain"
        unoptimized
        draggable={false}
      />
      <Image
        src={AWAKEN_NAMEPLATE_SPRITE_URL}
        alt=""
        width={220}
        height={96}
        className="pointer-events-none absolute top-[108.2%] left-[49.2%] w-[55%] -translate-x-1/2 -translate-y-1/2 object-contain"
        unoptimized
        draggable={false}
      />
      <Image
        src={iconicSrc}
        alt=""
        width={48}
        height={48}
        className="pointer-events-none absolute top-[107.3%] left-1/2 w-[14.4%] -translate-x-1/2 -translate-y-1/2 object-contain"
        unoptimized
        draggable={false}
      />
    </>
  );
}

type EquipmentDiamondProps = {
  tokens: EquipmentToken[];
  commanderId?: number;
};

export function EquipmentDiamond({ tokens, commanderId }: EquipmentDiamondProps) {
  const slots = new Map(tokens.map((token) => [token.slot, token]));
  const commanderTalents = getCommanderTalents(commanderId);

  return (
    <div
      className="relative mx-auto h-[calc(var(--gear-slot)*5.95)] w-[calc(var(--gear-slot)*3.05)] [--gear-slot:48px] sm:[--gear-slot:52px]"
      role="group"
      aria-label="Equipment"
    >
      {SLOT_POSITIONS.map((position) => (
        <GearSlot
          key={position.slot}
          position={position}
          token={slots.get(position.slot)}
          commanderTalents={commanderTalents}
        />
      ))}
    </div>
  );
}
