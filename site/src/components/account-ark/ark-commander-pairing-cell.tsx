import Image from "next/image";
import { getExtracted, getLocale } from "next-intl/server";
import { getCommanderName } from "@/lib/commander";

type ArkCommanderPairingCellProps = {
  primaryId: number | null | undefined;
  secondaryId: number | null | undefined;
};

function isValidCommanderId(id: number | null | undefined): id is number {
  return typeof id === "number" && Number.isFinite(id) && id > 0;
}

export async function ArkCommanderPairingCell({
  primaryId,
  secondaryId,
}: ArkCommanderPairingCellProps) {
  const t = await getExtracted();
  const locale = await getLocale();
  const unknownLabel = t("Unknown commander");
  const primaryName = isValidCommanderId(primaryId)
    ? (getCommanderName(primaryId, locale) ?? String(primaryId))
    : unknownLabel;
  const primarySrc = isValidCommanderId(primaryId)
    ? `https://cdn.rokbattles.com/game/commander/${primaryId}.png`
    : "https://cdn.rokbattles.com/game/ui/commander_unknown.png";

  const hasSecondary = isValidCommanderId(secondaryId);
  const secondaryName = hasSecondary
    ? (getCommanderName(secondaryId, locale) ?? String(secondaryId))
    : null;
  const secondarySrc = hasSecondary
    ? `https://cdn.rokbattles.com/game/commander/${secondaryId}.png`
    : null;

  return (
    <div className="flex flex-col">
      <span className="inline-flex items-center gap-2">
        <Image
          alt={t("{name} icon", { name: primaryName })}
          className="size-8 rounded-full object-cover"
          height={32}
          src={primarySrc}
          width={32}
        />
        <span>{primaryName}</span>
      </span>
      {secondarySrc ? (
        <span className="inline-flex items-center gap-2 text-zinc-600 dark:text-zinc-400">
          <Image
            alt={t("{name} icon", { name: secondaryName })}
            className="size-8 rounded-full object-cover"
            height={32}
            src={secondarySrc}
            width={32}
          />
          <span>{secondaryName}</span>
        </span>
      ) : null}
    </div>
  );
}
