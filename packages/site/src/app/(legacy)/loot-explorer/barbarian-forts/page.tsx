import { FortExplorer } from "@/components/loot-explorer/fort-explorer";
import { resolveLootExplorerSearchParams } from "@/lib/loot-explorer/search-params";

export default async function Page({ searchParams }: PageProps<"/loot-explorer/barbarian-forts">) {
  const parsed = await resolveLootExplorerSearchParams(searchParams);

  return <FortExplorer selectedType={parsed.type} selectedLevels={parsed.levels} />;
}
