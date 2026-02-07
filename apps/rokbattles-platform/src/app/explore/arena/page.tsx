import { getExtracted } from "next-intl/server";
import ExploreOlympianArenaTable from "@/components/explore/explore-olympian-arena-table";
import { Heading, Subheading } from "@/components/ui/heading";
import {
  Pagination,
  PaginationNext,
  PaginationPrevious,
} from "@/components/ui/pagination";
import { fetchExploreOlympianArena } from "@/data/fetch-explore-olympian-arena";
import { toInt } from "@/lib/params-helper";

export default async function Page({
  searchParams,
}: PageProps<"/explore/arena">) {
  const t = await getExtracted();
  const resolvedSearchParams = await searchParams;
  const page = Math.max(1, Math.trunc(toInt(resolvedSearchParams.page, 1)));
  const size = Math.min(
    100,
    Math.max(1, Math.trunc(toInt(resolvedSearchParams.size, 10)))
  );
  const reports = await fetchExploreOlympianArena(page, size);
  const hasNext = page * size < reports.total;
  const prevHref = page > 1 ? `?page=${page - 1}&size=${size}` : undefined;
  const nextHref = hasNext ? `?page=${page + 1}&size=${size}` : undefined;

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
