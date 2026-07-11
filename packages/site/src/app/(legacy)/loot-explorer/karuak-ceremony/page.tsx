import { KaruakCeremonyExplorer } from "@/components/loot-explorer/karuak-ceremony-explorer";
import { resolveLootExplorerSearchParams } from "@/lib/loot-explorer/search-params";

export default async function Page({ searchParams }: PageProps<"/loot-explorer/karuak-ceremony">) {
  const parsed = await resolveLootExplorerSearchParams(searchParams);
  return <KaruakCeremonyExplorer selectedType={parsed.type} />;
}
