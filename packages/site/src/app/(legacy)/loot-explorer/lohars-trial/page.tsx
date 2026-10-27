import { LoharTrialExplorer } from "@/components/loot-explorer/lohar-trial-explorer";
import { resolveLootExplorerSearchParams } from "@/lib/loot-explorer/search-params";

export default async function Page({ searchParams }: PageProps<"/loot-explorer/lohars-trial">) {
  const parsed = await resolveLootExplorerSearchParams(searchParams);
  return <LoharTrialExplorer selectedType={parsed.type} />;
}
