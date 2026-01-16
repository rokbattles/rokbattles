"use client";

import Image from "next/image";
import { useTranslations } from "next-intl";
import { getEquipmentName } from "@/hooks/use-equipment-name";
import type { EquipmentToken } from "@/lib/report/parsers";

type ReportEquipmentSlotProps = {
  token?: EquipmentToken;
};

export function ReportEquipmentSlot({ token }: ReportEquipmentSlotProps) {
  const t = useTranslations("report");
  const { tier, isSpecialTalent } = getTierInfo(token?.attr);
  const tierLabel = tier != null ? toRomanNumeral(tier) : null;
  const label =
    token?.id != null
      ? (getEquipmentName(token.id) ?? token.id.toString())
      : undefined;
  const equipmentAlt =
    token?.id != null ? t("equipment.slotAlt", { id: token.id }) : undefined;

  return (
    <div
      className="relative h-14 w-14 select-none overflow-hidden rounded-lg bg-zinc-600/10 sm:h-16 sm:w-16 dark:bg-white/5"
      title={label}
    >
      {token?.id ? (
        <Image
          alt={equipmentAlt ?? ""}
          className="object-contain"
          fill
          sizes="(min-width: 640px) 64px, 56px"
          src={`/lilith/images/equipment/${token.id}.png`}
        />
      ) : (
        <div className="flex h-full w-full items-center justify-center font-semibold text-[10px] text-zinc-300">
          -
        </div>
      )}
      {tierLabel ? (
        <span className="absolute bottom-1 left-1 rounded bg-black/70 px-1 font-semibold text-[0.625rem] text-white">
          {tierLabel}
        </span>
      ) : null}
      {isSpecialTalent ? (
        <span className="absolute right-1 bottom-1 rounded bg-amber-500 px-1 font-semibold text-[0.625rem] text-white">
          ST
        </span>
      ) : null}
    </div>
  );
}

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
