"use client";

import Image from "next/image";
import type { CSSProperties } from "react";

import { getEquipmentName } from "@/hooks/use-equipment-name";
import {
  AWAKEN_FRAME_SPRITE_URL,
  AWAKEN_NAMEPLATE_SPRITE_URL,
  type EquipmentRarity,
  getEquipmentRarity,
  getEquipmentSpriteUrl,
  getEquipmentTierInfo,
  getEquipmentTroopTypeIconSrc,
  getIconicTierSpriteUrl,
  toRomanNumeral,
} from "@/lib/equipment";
import type { EquipmentToken } from "@/lib/report/parsers";

// Report equipment slot numbers, laid out as a diamond of six rows.
// Slots: 1 weapon, 2 helmet, 3 chest, 4 gloves, 5 legs, 6 boots, 7 and 8 accessories.
const DIAMOND_ROWS: number[][] = [[2], [3], [1, 4], [5], [7, 8], [6]];

// RGB triples for the glow behind gear that is not legendary.
const GLOW_RGB: Record<Exclude<EquipmentRarity, "legendary">, string> = {
  normal: "255,255,255",
  advanced: "74,222,128",
  elite: "79,163,240",
  epic: "176,107,224",
};

// The nameplate image is 220 x 96 and its gold diamond is centered at (110, 42).
const NAMEPLATE_ASPECT = 96 / 220;
const NAMEPLATE_DIAMOND_Y = 42 / 96;
// The old iconic badge was centered 23px below the middle of a 52px slot.
const NAMEPLATE_DIAMOND_TOP = 0.5 + 23 / 52;

type GearSlotProps = {
  token?: EquipmentToken;
};

function GearSlot({ token }: GearSlotProps) {
  const rarity = getEquipmentRarity(token?.id);
  const spriteUrl = getEquipmentSpriteUrl(token?.id);
  const { tier, isSpecialTalent, troopType } = getEquipmentTierInfo(token?.attr);
  const name = token?.id != null ? (getEquipmentName(token.id) ?? token.id.toString()) : undefined;
  const isLegendary = rarity === "legendary";
  const iconicSrc = isLegendary ? getIconicTierSpriteUrl(tier) : undefined;
  const talentSrc = isSpecialTalent ? getEquipmentTroopTypeIconSrc(troopType) : undefined;
  const title = [
    name,
    isLegendary && tier != null ? `Iconic ${toRomanNumeral(tier) ?? tier}` : undefined,
    troopType ? `${troopType} talent` : undefined,
  ]
    .filter(Boolean)
    .join(" – ");

  if (!token || !spriteUrl) {
    return (
      <span
        className="relative block size-(--gear-slot) shrink-0 select-none"
        title={name}
        aria-hidden={name ? undefined : true}
      >
        <span className="absolute inset-[22%] rotate-45 rounded-[12%] border border-white/15" />
      </span>
    );
  }

  return (
    <span className="relative block size-(--gear-slot) shrink-0 select-none" title={title}>
      {isLegendary ? (
        <Image
          src={AWAKEN_FRAME_SPRITE_URL}
          alt=""
          fill
          sizes="64px"
          className="rotate-45 scale-[.82] object-contain"
          unoptimized
          draggable={false}
        />
      ) : rarity ? (
        <span
          className="pointer-events-none absolute inset-[-6%] rounded-full"
          style={
            {
              background: `radial-gradient(circle, rgba(${GLOW_RGB[rarity]},.55) 0%, rgba(${GLOW_RGB[rarity]},.28) 42%, rgba(${GLOW_RGB[rarity]},0) 72%)`,
            } as CSSProperties
          }
        />
      ) : null}
      <Image
        src={spriteUrl}
        alt={name ? `Equipment: ${name}` : ""}
        width={64}
        height={64}
        className={`absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 object-contain ${
          isLegendary ? "size-[62%]" : "size-[78%]"
        }`}
        unoptimized
        draggable={false}
      />
      {iconicSrc ? (
        // The nameplate sits where the iconic badge did: its gold diamond is centered 23px
        // below the middle of a 52px slot. The tier number is centered on that diamond.
        <span
          className="pointer-events-none absolute left-1/2 z-10 block w-(--gear-nameplate)"
          style={{
            aspectRatio: String(1 / NAMEPLATE_ASPECT),
            top: `calc(var(--gear-slot) * ${String(NAMEPLATE_DIAMOND_TOP)} - var(--gear-nameplate) * ${String(NAMEPLATE_ASPECT * NAMEPLATE_DIAMOND_Y)})`,
            transform: "translateX(-50%)",
          }}
        >
          <Image
            src={AWAKEN_NAMEPLATE_SPRITE_URL}
            alt=""
            fill
            sizes="64px"
            className="object-contain"
            unoptimized
            draggable={false}
          />
          <Image
            src={iconicSrc}
            alt=""
            width={48}
            height={48}
            className="absolute left-1/2 w-[23%] -translate-x-1/2 -translate-y-1/2 object-contain"
            style={{ top: `${String(NAMEPLATE_DIAMOND_Y * 100)}%` }}
            unoptimized
            draggable={false}
          />
        </span>
      ) : null}
      {talentSrc ? (
        <Image
          src={talentSrc}
          alt=""
          width={20}
          height={20}
          className="pointer-events-none absolute top-[65%] left-[65%] z-10 w-[31%] -translate-x-1/2 -translate-y-1/2 object-contain"
          unoptimized
          draggable={false}
        />
      ) : null}
    </span>
  );
}

type ReportGearDiamondProps = {
  tokens: EquipmentToken[];
};

export function ReportGearDiamond({ tokens }: ReportGearDiamondProps) {
  const slots = tokens.reduce<Record<number, EquipmentToken | undefined>>((acc, token) => {
    acc[token.slot] = token;
    return acc;
  }, {});

  return (
    <div
      className="flex flex-col items-center pb-[calc(var(--gear-slot)*0.13)] [--gear-nameplate:calc(var(--gear-slot)*0.76)] [--gear-slot:62px] sm:[--gear-slot:68px]"
      role="group"
      aria-label="Equipment"
    >
      {DIAMOND_ROWS.map((row, index) => {
        const previous = DIAMOND_ROWS[index - 1];
        const isPair = row.length > 1;
        // Rows overlap the one above by 20 / 52 of a slot, which turns the stack into a diamond.
        // Two single slots in a row do not overlap, so they get a small gap instead.
        const spacing =
          index === 0
            ? ""
            : !isPair && previous && previous.length === 1
              ? "mt-[calc(var(--gear-slot)*3/52)]"
              : "mt-[calc(var(--gear-slot)*-20/52)]";

        return (
          <div
            key={row.join("-")}
            className={`flex justify-center gap-[calc(var(--gear-slot)*6/52)] ${spacing}`}
          >
            {row.map((slot) => (
              <GearSlot key={slot} token={slots[slot]} />
            ))}
          </div>
        );
      })}
    </div>
  );
}
