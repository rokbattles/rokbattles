"use client";

import { CommanderIcon } from "@/components/commander-icon";
import { getCommanderName } from "@/hooks/use-commander-name";

type Props = {
  primaryId: number | null | undefined;
  primaryAwakened?: boolean | null;
  primarySkinId?: number | null;
  secondaryId: number | null | undefined;
  secondaryAwakened?: boolean | null;
  secondarySkinId?: number | null;
};

function isValidCommanderId(id: number | null | undefined): id is number {
  return typeof id === "number" && Number.isFinite(id) && id > 0;
}

export default function ParticipantCell({
  primaryId,
  primaryAwakened,
  primarySkinId,
  secondaryId,
  secondaryAwakened,
  secondarySkinId,
}: Props) {
  const unknownLabel = "Unknown commander";

  const primaryName = isValidCommanderId(primaryId)
    ? (getCommanderName(primaryId) ?? String(primaryId))
    : unknownLabel;

  const hasSecondary = isValidCommanderId(secondaryId);
  const secondaryName = hasSecondary
    ? (getCommanderName(secondaryId) ?? String(secondaryId))
    : null;

  return (
    <span className="inline-flex items-center align-middle">
      <span title={primaryName}>
        <CommanderIcon
          alt={`${primaryName} icon`}
          awakened={primaryAwakened}
          className="size-8"
          skinId={primarySkinId}
          id={primaryId}
        />
      </span>
      {hasSecondary ? (
        <span title={secondaryName ?? undefined}>
          <CommanderIcon
            alt={`${secondaryName} icon`}
            awakened={secondaryAwakened}
            className="size-8"
            skinId={secondarySkinId}
            id={secondaryId}
          />
        </span>
      ) : null}
    </span>
  );
}
