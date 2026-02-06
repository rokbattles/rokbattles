import { getExtracted } from "next-intl/server";
import ExploreBattleReportsTable from "@/components/explore/explore-battle-reports-table";
import { Heading, Subheading } from "@/components/ui/heading";
import {
  Pagination,
  PaginationNext,
  PaginationPrevious,
} from "@/components/ui/pagination";
import { fetchExploreBattleReports } from "@/data/fetch-explore-battle-reports";
import { toInt } from "@/lib/params-helper";

export default async function Page({ searchParams }: PageProps<"/explore">) {
  const t = await getExtracted();
  const resolvedSearchParams = await searchParams;
  const page = Math.max(1, Math.trunc(toInt(resolvedSearchParams.page, 1)));
  const size = Math.min(
    100,
    Math.max(1, Math.trunc(toInt(resolvedSearchParams.size, 10)))
  );
  const reports = await fetchExploreBattleReports(page, size);
  const hasNext = page * size < reports.total;
  const prevHref = page > 1 ? `?page=${page - 1}&size=${size}` : undefined;
  const nextHref = hasNext ? `?page=${page + 1}&size=${size}` : undefined;

  return (
    <>
      <Heading>{t("Battle Reports")}</Heading>
      <Subheading className="mt-8">{t("Live feed")}</Subheading>
      <ExploreBattleReportsTable reports={reports} />
      <Pagination className="mt-4">
        <PaginationPrevious href={prevHref} />
        <PaginationNext href={nextHref} />
      </Pagination>
    </>
  );
}
