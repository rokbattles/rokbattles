import { FortExplorer } from "@/components/loot-explorer/fort-explorer";
import { resolveLootExplorerSearchParams } from "@/lib/loot-explorer/search-params";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default async function Page({ searchParams }: PageProps<"/loot-explorer/barbarian-forts">) {
  const parsed = await resolveLootExplorerSearchParams(searchParams);

  return <FortExplorer selectedType={parsed.type} selectedLevels={parsed.levels} />;
}
