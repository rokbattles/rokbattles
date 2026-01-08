"use client";

import { getCommanderName } from "@/hooks/useCommanderName";

type Props = {
  primaryId: number;
  secondaryId: number;
};

export default function ParticipantCell({ primaryId, secondaryId }: Props) {
  return (
    <div className="flex flex-col gap-1">
      <span>{getCommanderName(primaryId) ?? primaryId}</span>
      {secondaryId > 0 && (
        <span className="text-zinc-600 dark:text-zinc-400">
          {getCommanderName(secondaryId) ?? secondaryId}
        </span>
      )}
    </div>
  );
}
