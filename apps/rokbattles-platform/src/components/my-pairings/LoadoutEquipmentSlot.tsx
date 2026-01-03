"use client";

import Image from "next/image";
import { useTranslations } from "next-intl";
import { getEquipmentName } from "@/hooks/useEquipmentName";
import type { LoadoutSnapshot } from "@/hooks/usePairings";

type LoadoutEquipmentSlotProps = {
  token?: LoadoutSnapshot["equipment"][number];
};

function getTierInfo(attr?: number) {
  if (typeof attr !== "number" || !Number.isFinite(attr)) {
    return { tier: undefined, isSpecialTalent: false };
  }

  const numeric = Number(attr);
  const isSpecialTalent = numeric >= 10;
  const base = isSpecialTalent ? numeric % 10 : numeric;
  const tier = Number.isFinite(base) ? base : undefined;

  return { tier, isSpecialTalent };
}

function toRomanNumeral(value: number | undefined) {
  if (typeof value !== "number") {
    return null;
  }

  const numerals = ["", "I", "II", "III", "IV", "V"];
  return numerals[value] ?? null;
}

export function LoadoutEquipmentSlot({ token }: LoadoutEquipmentSlotProps) {
  const t = useTranslations("pairings");
  const { tier, isSpecialTalent } = getTierInfo(token?.attr);
  const tierLabel = tier != null ? toRomanNumeral(tier) : null;
  const label =
    token?.id != null
      ? (getEquipmentName(token.id) ?? token.id.toString())
      : t("labels.emptyEquipment");

  return (
    <div
      className="relative h-12 w-12 select-none overflow-hidden rounded-lg bg-zinc-600/10 dark:bg-white/5 sm:h-14 sm:w-14"
      title={label}
    >
      {token?.id ? (
        <Image
          src={`/lilith/images/equipment/${token.id}.png`}
          alt={label}
          fill
          sizes="(min-width: 640px) 56px, 48px"
          className="object-contain"
        />
      ) : (
        <div className="flex h-full w-full items-center justify-center text-[10px] font-semibold text-zinc-300">
          -
        </div>
      )}
      {tierLabel ? (
        <span className="absolute bottom-1 left-1 rounded bg-black/70 px-1 text-[0.625rem] font-semibold text-white">
          {tierLabel}
        </span>
      ) : null}
      {isSpecialTalent ? (
        <span className="absolute bottom-1 right-1 rounded bg-amber-500 px-1 text-[0.625rem] font-semibold text-white">
          ST
        </span>
      ) : null}
    </div>
  );
}
