"use client";

import { useCommanderName } from "@/hooks/useCommanderName";

type Props = {
  primaryId: number;
  secondaryId: number;
};

function parse(v: number | undefined) {
  return typeof v === "number" && Number.isFinite(v) && v > 0 ? String(v) : undefined;
}

export default function ParticipantCell({ primaryId, secondaryId }: Props) {
  const primaryName = useCommanderName(primaryId) ?? parse(primaryId);
  const secondaryName = useCommanderName(secondaryId) ?? parse(secondaryId);

  const hasPrimary = Boolean(primaryName);
  const hasSecondary = Boolean(secondaryName);

  if (!hasPrimary && !hasSecondary) {
    return <span className="text-zinc-500 dark:text-zinc-400">Unknown</span>;
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
