import { BarbarianExplorer } from "@/components/loot-explorer/barbarian-explorer";
import { resolveLootExplorerSearchParams } from "@/lib/loot-explorer/search-params";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default async function Page({ searchParams }: PageProps<"/loot-explorer/barbarians">) {
  const parsed = await resolveLootExplorerSearchParams(searchParams);

  return <BarbarianExplorer selectedType={parsed.type} selectedLevels={parsed.levels} />;
}
