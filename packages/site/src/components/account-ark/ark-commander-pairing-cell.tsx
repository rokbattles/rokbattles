"use client";

import type { ReactElement } from "react";
import { CommanderIcon } from "@/components/commander-icon";
import { getCommanderName } from "@/lib/commander";

type ArkCommanderPairingCellProps = {
  primaryId: number | null | undefined;
  secondaryId: number | null | undefined;
};

function isValidCommanderId(id: number | null | undefined): id is number {
  return typeof id === "number" && Number.isFinite(id) && id > 0;
}

export function ArkCommanderPairingCell({
  primaryId,
  secondaryId,
}: ArkCommanderPairingCellProps): ReactElement {
  const unknownLabel = "Unknown commander";
  const primaryName = isValidCommanderId(primaryId)
    ? (getCommanderName(primaryId) ?? String(primaryId))
    : unknownLabel;

  const hasSecondary = isValidCommanderId(secondaryId);
  const secondaryName = hasSecondary
    ? (getCommanderName(secondaryId) ?? String(secondaryId))
    : null;

  return (
    <div className="flex items-center gap-2">
      <span title={primaryName}>
        <CommanderIcon alt={`${primaryName} icon`} className="size-8 rounded-full" id={primaryId} />
      </span>
      {hasSecondary ? (
        <span title={secondaryName ?? undefined}>
          <CommanderIcon
            alt={`${secondaryName} icon`}
            className="size-8 rounded-full"
            id={secondaryId}
          />
        </span>
      ) : null}
    </div>
  );
}
