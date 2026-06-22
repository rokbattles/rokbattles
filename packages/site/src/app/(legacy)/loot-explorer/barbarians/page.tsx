import { BarbarianExplorer } from "@/components/loot-explorer/barbarian-explorer";
import { resolveLootExplorerSearchParams } from "@/lib/loot-explorer/search-params";

export default async function Page({ searchParams }: PageProps<"/loot-explorer/barbarians">) {
  const parsed = await resolveLootExplorerSearchParams(searchParams);

  return <BarbarianExplorer selectedType={parsed.type} selectedLevels={parsed.levels} />;
}
