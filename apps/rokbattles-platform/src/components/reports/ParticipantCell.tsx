"use client";

import { useTranslations } from "next-intl";
import { getCommanderName } from "@/hooks/useCommanderName";

type Props = {
  primaryId: number;
  secondaryId: number;
};

function parse(v: number | undefined) {
  return typeof v === "number" && Number.isFinite(v) ? String(v) : undefined;
}

export default function ParticipantCell({ primaryId, secondaryId }: Props) {
  const t = useTranslations("common");
  const primaryName = getCommanderName(primaryId) ?? parse(primaryId);
  const secondaryName = getCommanderName(secondaryId) ?? parse(secondaryId);

  const hasPrimary = Boolean(primaryName);
  const hasSecondary = Boolean(secondaryName);

  if (!hasPrimary && !hasSecondary) {
    return <span className="text-zinc-500 dark:text-zinc-400">{t("labels.unknown")}</span>;
  }

  return (
    <div className="flex flex-col gap-1">
      {hasPrimary ? <span>{primaryName}</span> : null}
      {hasSecondary ? (
        <span className="text-zinc-600 dark:text-zinc-400">{secondaryName}</span>
      ) : null}
    </div>
  );
}
