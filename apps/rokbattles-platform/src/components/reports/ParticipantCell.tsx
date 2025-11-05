"use client";

import { useCommanderName } from "@/hooks/useCommanderName";

type Props = {
  primaryId: number;
  secondaryId: number;
};

export default function ParticipantCell({ primaryId, secondaryId }: Props) {
  const primaryName = useCommanderName(primaryId) ?? String(primaryId);
  const secondaryName = useCommanderName(secondaryId) ?? String(secondaryId);

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
