import { getExtracted } from "next-intl/server";
import ExploreBattleReportsTable from "@/components/explore/explore-battle-reports-table";
import { Heading, Subheading } from "@/components/ui/heading";
import {
  Pagination,
  PaginationNext,
  PaginationPrevious,
} from "@/components/ui/pagination";
import { fetchExploreBattleReports } from "@/data/fetch-explore-battle-reports";
import { toStr } from "@/lib/params-helper";

export default async function Page({ searchParams }: PageProps<"/explore">) {
  const t = await getExtracted();
  const resolvedSearchParams = await searchParams;
  const after = toStr(resolvedSearchParams.after);
  const before = toStr(resolvedSearchParams.before);
  const reports = await fetchExploreBattleReports(after, before);
  const prevHref = reports.previousBefore
    ? `?before=${encodeURIComponent(reports.previousBefore)}`
    : undefined;
  const nextHref = reports.nextAfter
    ? `?after=${encodeURIComponent(reports.nextAfter)}`
    : undefined;

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
