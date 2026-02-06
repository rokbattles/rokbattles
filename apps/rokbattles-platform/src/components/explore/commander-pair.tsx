import { getCommanderNameOrId } from "@/lib/commander-names";

export default function CommanderPair({
  primary,
  secondary,
}: {
  primary: number | null;
  secondary: number | null;
}) {
  const primaryLabel = getCommanderNameOrId(primary);
  const secondaryLabel = getCommanderNameOrId(secondary);

  return (
    <div className="flex flex-col gap-1">
      <span className="tabular-nums">{primaryLabel}</span>
      {secondaryLabel !== undefined ? (
        <span className="text-zinc-600 tabular-nums dark:text-zinc-400">
          {secondaryLabel}
        </span>
      ) : null}
    </div>
  );
}
