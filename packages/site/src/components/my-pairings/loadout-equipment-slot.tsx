"use client";

import Image from "next/image";
import { useExtracted } from "next-intl";
import { getEquipmentName } from "@/hooks/use-equipment-name";
import {
  getEquipmentSpriteUrl,
  getEquipmentTierInfo,
  getEquipmentTroopTypeIconSrc,
  toRomanNumeral,
} from "@/lib/equipment";
import type { LoadoutSnapshot } from "@/lib/pairings";

type LoadoutEquipmentSlotProps = {
  token?: LoadoutSnapshot["equipment"][number];
};

export function LoadoutEquipmentSlot({ token }: LoadoutEquipmentSlotProps) {
  const t = useExtracted();
  const { tier, isSpecialTalent, troopType } = getEquipmentTierInfo(token?.attr);
  const tierLabel = tier != null ? toRomanNumeral(tier) : null;
  const label =
    token?.id != null ? (getEquipmentName(token.id) ?? token.id.toString()) : t("Empty");
  const troopTypeIconSrc = getEquipmentTroopTypeIconSrc(troopType);
  const equipmentSpriteUrl = getEquipmentSpriteUrl(token?.id);

  return (
    <div
      className="relative h-12 w-12 select-none overflow-hidden rounded-lg bg-zinc-600/10 dark:bg-white/5 sm:h-14 sm:w-14"
      title={label}
    >
      {equipmentSpriteUrl ? (
        <Image
          src={equipmentSpriteUrl}
          alt={label}
          fill
          sizes="(min-width: 640px) 56px, 48px"
          className="object-contain"
          unoptimized
        />
      ) : (
        <div className="flex h-full w-full items-center justify-center text-[10px] font-semibold text-zinc-300">
          -
        </div>
      )}
      {tierLabel ? (
        <span className="absolute bottom-1 left-1 rounded bg-black/70 px-1 text-xs font-semibold text-white">
          {tierLabel}
        </span>
      ) : null}
      {isSpecialTalent ? (
        <span className="absolute right-0.5 bottom-0.5 flex h-5 w-5 items-center justify-center">
          {troopTypeIconSrc ? (
            <Image
              src={troopTypeIconSrc}
              alt={t("Special talent {troopType}", { troopType })}
              width={20}
              height={20}
              className="h-5 w-5 object-contain"
            />
          ) : null}
        </span>
      ) : null}
    </div>
  );
}
