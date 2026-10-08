"use client";

import { useExtracted, useLocale } from "next-intl";
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
  const t = useExtracted();
  const locale = useLocale();

  const unknownLabel = t("Unknown commander");
  const primaryName = isValidCommanderId(primaryId)
    ? (getCommanderName(primaryId, locale) ?? String(primaryId))
    : unknownLabel;

  const hasSecondary = isValidCommanderId(secondaryId);
  const secondaryName = hasSecondary
    ? (getCommanderName(secondaryId, locale) ?? String(secondaryId))
    : null;

  return (
    <div className="flex items-center gap-2">
      <span title={primaryName}>
        <CommanderIcon
          alt={t("{name} icon", { name: primaryName })}
          className="size-8 rounded-full"
          id={primaryId}
        />
      </span>
      {hasSecondary ? (
        <span title={secondaryName ?? undefined}>
          <CommanderIcon
            alt={t("{name} icon", { name: secondaryName })}
            className="size-8 rounded-full"
            id={secondaryId}
          />
        </span>
      ) : null}
    </div>
  );
}
