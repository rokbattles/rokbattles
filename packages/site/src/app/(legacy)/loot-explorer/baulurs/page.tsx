import { BaulurExplorer } from "@/components/loot-explorer/baulur-explorer";
import { resolveLootExplorerSearchParams } from "@/lib/loot-explorer/search-params";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default async function Page({ searchParams }: PageProps<"/loot-explorer/baulurs">) {
  const parsed = await resolveLootExplorerSearchParams(searchParams);

  return <BaulurExplorer selectedType={parsed.type} />;
}
