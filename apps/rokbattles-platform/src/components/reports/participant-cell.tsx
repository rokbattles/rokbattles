"use client";

import Image from "next/image";
import { useTranslations } from "next-intl";
import { getCommanderName } from "@/hooks/use-commander-name";

type Props = {
  primaryId: number | null | undefined;
  secondaryId: number | null | undefined;
};

function isValidCommanderId(id: number | null | undefined): id is number {
  return typeof id === "number" && Number.isFinite(id) && id > 0;
}

export default function ParticipantCell({ primaryId, secondaryId }: Props) {
  const tCommon = useTranslations("common");
  const unknownLabel = tCommon("labels.unknownCommander");

  const primaryName = isValidCommanderId(primaryId)
    ? (getCommanderName(primaryId) ?? String(primaryId))
    : unknownLabel;
  const primarySrc = isValidCommanderId(primaryId)
    ? `/game/commander/${primaryId}.png`
    : "/game/ui/commander_unknown.png";

  const hasSecondary = isValidCommanderId(secondaryId);
  const secondaryName = hasSecondary
    ? (getCommanderName(secondaryId) ?? String(secondaryId))
    : null;
  const secondarySrc = hasSecondary ? `/game/commander/${secondaryId}.png` : null;

  return (
    <div className="flex flex-col">
      <span className="inline-flex items-center gap-2">
        <Image
          alt={tCommon("alt.namedIcon", { name: primaryName })}
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
            alt={tCommon("alt.namedIcon", { name: secondaryName })}
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
