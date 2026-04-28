"use client";

import Image from "next/image";
import { useExtracted } from "next-intl";
import { getEquipmentName } from "@/hooks/use-equipment-name";
import {
  getEquipmentTierInfo,
  getEquipmentTroopTypeIconSrc,
  toRomanNumeral,
} from "@/lib/equipment";
import type { EquipmentToken } from "@/lib/report/parsers";

type ReportEquipmentSlotProps = {
  token?: EquipmentToken;
};

export function ReportEquipmentSlot({ token }: ReportEquipmentSlotProps) {
  const t = useExtracted();
  const { tier, isSpecialTalent, troopType } = getEquipmentTierInfo(token?.attr);
  const tierLabel = tier != null ? toRomanNumeral(tier) : null;
  const label = token?.id != null ? (getEquipmentName(token.id) ?? token.id.toString()) : undefined;
  const equipmentAlt =
    token?.id != null ? t("Equipment {id}", { id: token.id.toString() }) : undefined;
  const troopTypeIconSrc = getEquipmentTroopTypeIconSrc(troopType);

  return (
    <div
      className="relative h-14 w-14 select-none overflow-hidden rounded-lg bg-zinc-600/10 dark:bg-white/5 sm:h-16 sm:w-16"
      title={label}
    >
      {token?.id ? (
        <Image
          src={`https://cdn.rokbattles.com/game/equipment/${token.id}.png`}
          alt={equipmentAlt ?? ""}
          fill
          sizes="(min-width: 640px) 64px, 56px"
          className="object-contain"
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
