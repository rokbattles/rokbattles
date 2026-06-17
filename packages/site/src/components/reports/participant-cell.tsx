"use client";

import { useExtracted } from "next-intl";
import { CommanderIcon } from "@/components/commander-icon";
import { getCommanderName } from "@/hooks/use-commander-name";

type Props = {
  primaryId: number | null | undefined;
  primaryAwakened?: boolean | null;
  secondaryId: number | null | undefined;
  secondaryAwakened?: boolean | null;
};

function isValidCommanderId(id: number | null | undefined): id is number {
  return typeof id === "number" && Number.isFinite(id) && id > 0;
}

export default function ParticipantCell({
  primaryId,
  primaryAwakened,
  secondaryId,
  secondaryAwakened,
}: Props) {
  const t = useExtracted();
  const unknownLabel = t("Unknown commander");

  const primaryName = isValidCommanderId(primaryId)
    ? (getCommanderName(primaryId) ?? String(primaryId))
    : unknownLabel;

  const hasSecondary = isValidCommanderId(secondaryId);
  const secondaryName = hasSecondary
    ? (getCommanderName(secondaryId) ?? String(secondaryId))
    : null;

  return (
    <div className="flex flex-col">
      <span className="inline-flex items-center gap-2">
        <CommanderIcon
          alt={t("{name} icon", { name: primaryName })}
          awakened={primaryAwakened}
          className="size-8 rounded-full"
          id={primaryId}
        />
        <span>{primaryName}</span>
      </span>
      {hasSecondary ? (
        <span className="inline-flex items-center gap-2 text-zinc-600 dark:text-zinc-400">
          <CommanderIcon
            alt={t("{name} icon", { name: secondaryName })}
            awakened={secondaryAwakened}
            className="size-8 rounded-full"
            id={secondaryId}
          />
          <span>{secondaryName}</span>
        </span>
      ) : null}
    </div>
  );
}
