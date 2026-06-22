import { BaulurExplorer } from "@/components/loot-explorer/baulur-explorer";
import { resolveLootExplorerSearchParams } from "@/lib/loot-explorer/search-params";

export default async function Page({ searchParams }: PageProps<"/loot-explorer/baulurs">) {
  const parsed = await resolveLootExplorerSearchParams(searchParams);

  return <BaulurExplorer selectedType={parsed.type} />;
}
