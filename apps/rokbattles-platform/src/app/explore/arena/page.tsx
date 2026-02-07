import { getExtracted } from "next-intl/server";
import ExploreOlympianArenaTable from "@/components/explore/explore-olympian-arena-table";
import { Heading, Subheading } from "@/components/ui/heading";
import {
  Pagination,
  PaginationNext,
  PaginationPrevious,
} from "@/components/ui/pagination";
import { fetchExploreOlympianArena } from "@/data/fetch-explore-olympian-arena";
import { toStr } from "@/lib/params-helper";

export default async function Page({
  searchParams,
}: PageProps<"/explore/arena">) {
  const t = await getExtracted();
  const resolvedSearchParams = await searchParams;
  const after = toStr(resolvedSearchParams.after);
  const before = toStr(resolvedSearchParams.before);
  const reports = await fetchExploreOlympianArena(after, before);
  const prevHref = reports.previousBefore
    ? `?before=${encodeURIComponent(reports.previousBefore)}`
    : undefined;
  const nextHref = reports.nextAfter
    ? `?after=${encodeURIComponent(reports.nextAfter)}`
    : undefined;

  return (
    <>
      <Heading>{t("Olympian Arena")}</Heading>
      <Subheading className="mt-8">{t("Live feed")}</Subheading>
      <ExploreOlympianArenaTable reports={reports} />
      <Pagination className="mt-4">
        <PaginationPrevious href={prevHref} />
        <PaginationNext href={nextHref} />
      </Pagination>
    </>
  );
}
