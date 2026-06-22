import { LootExplorerLayout, type LootExplorerSection } from "./loot-explorer-layout";

export function LootExplorerStatus({
  active,
  message,
}: {
  active: LootExplorerSection;
  message: string;
}) {
  return (
    <LootExplorerLayout active={active}>
      <p className="text-sm/6 text-zinc-500 dark:text-zinc-400">{message}</p>
    </LootExplorerLayout>
  );
}
