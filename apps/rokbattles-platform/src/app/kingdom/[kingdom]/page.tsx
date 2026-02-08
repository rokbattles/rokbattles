import { getExtracted } from "next-intl/server";
import KingdomGovernorsTable from "@/components/kingdoms/kingdom-governors-table";
import { Heading, Subheading } from "@/components/ui/heading";
import {
  Pagination,
  PaginationNext,
  PaginationPrevious,
} from "@/components/ui/pagination";
import { fetchKingdomGovernorDataList } from "@/data/fetch-kingdom-governor-data-list";
import { toInt } from "@/lib/params-helper";

export default async function Page({
  params,
  searchParams,
}: PageProps<"/kingdom/[kingdom]">) {
  const t = await getExtracted();
  const [resolvedParams, resolvedSearchParams] = await Promise.all([
    params,
    searchParams,
  ]);
  const { kingdom } = resolvedParams;
  const page = Math.max(1, Math.trunc(toInt(resolvedSearchParams.page, 1)));
  const size = Math.min(
    100,
    Math.max(1, Math.trunc(toInt(resolvedSearchParams.size, 10)))
  );
  const governors = await fetchKingdomGovernorDataList(
    toInt(kingdom, 0),
    page,
    size
  );
  const offset = (page - 1) * size;
  const hasNext = page * size < governors.total;
  const prevHref = page > 1 ? `?page=${page - 1}&size=${size}` : undefined;
  const nextHref = hasNext ? `?page=${page + 1}&size=${size}` : undefined;

  return (
    <>
      <Heading>{t("Governors")}</Heading>
      <Subheading className="mt-8">{t("Overview")}</Subheading>
      <KingdomGovernorsTable governors={governors} offset={offset} />
      <Pagination className="mt-4">
        <PaginationPrevious href={prevHref} />
        <PaginationNext href={nextHref} />
      </Pagination>
    </>
  );
}
