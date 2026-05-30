"use client";

import Image from "next/image";
import { useExtracted } from "next-intl";
import { toRomanNumeral } from "@/lib/equipment";
import { getRelicLevelFromId, getRelicName, getRelicSpriteUrls } from "@/lib/relic";
import type { RawRelicInfo } from "@/lib/types/raw-report";

type ReportRelicSlotProps = {
  relic: RawRelicInfo;
};

export function ReportRelicSlot({ relic }: ReportRelicSlotProps) {
  const t = useExtracted();
  const id = relic.id;
  const label = getRelicName(id) ?? id?.toString();
  const levelLabel = toRomanNumeral(getRelicLevelFromId(id)) ?? "I";
  const spriteUrls = getRelicSpriteUrls(id);

  return (
    <div
      className="relative h-14 w-14 select-none overflow-hidden rounded-lg bg-zinc-600/10 dark:bg-white/5 sm:h-16 sm:w-16"
      title={label}
    >
      {spriteUrls?.length ? (
        spriteUrls.map((spriteUrl) => (
          <Image
            key={spriteUrl}
            src={spriteUrl}
            alt={id != null ? t("Relic {id}", { id: id.toString() }) : ""}
            fill
            sizes="(min-width: 640px) 64px, 56px"
            className="object-contain"
          />
        ))
      ) : (
        <div className="flex h-full w-full items-center justify-center text-[10px] font-semibold text-zinc-300">
          -
        </div>
      )}
      <span className="absolute bottom-1 left-1 rounded bg-black/70 px-1 text-xs font-semibold text-white">
        {levelLabel}
      </span>
    </div>
  );
}
