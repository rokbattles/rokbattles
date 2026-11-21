import type { Metadata } from "next";
import { ProgramApplicationsTable } from "@/components/(app)/admin/program-applications-table";
import { Heading } from "@/components/ui/heading";
import { Pagination, PaginationNext, PaginationPrevious } from "@/components/ui/pagination";
import { Text } from "@/components/ui/text";
import { getAdminProgramApplications } from "@/lib/(app)/get-admin-program-applications";

export const metadata: Metadata = { title: "Program Applications" };

function parsePage(rawPage: string | string[] | undefined) {
  const page = Number(rawPage);

  return Number.isInteger(page) && page > 0 && page <= 4_294_967_295 ? page : 1;
}

export default async function Page({ searchParams }: PageProps<"/admin/program-applications">) {
  const page = parsePage((await searchParams).page);
  const { applications, hasNextPage } = await getAdminProgramApplications(page);
  const pageHref = (target: number) => `/admin/program-applications?page=${target}`;

  return (
    <div className="space-y-6">
      <Heading>Program Applications</Heading>
      {applications.length > 0 ? (
        <ProgramApplicationsTable key={page} applications={applications} />
      ) : (
        <Text>{page === 1 ? "No applications yet." : "No applications on this page."}</Text>
      )}
      {page > 1 || hasNextPage ? (
        <Pagination className="items-center">
          <PaginationPrevious href={page > 1 ? pageHref(page - 1) : null} />
          <Text>Page {page}</Text>
          <PaginationNext href={hasNextPage ? pageHref(page + 1) : null} />
        </Pagination>
      ) : null}
    </div>
  );
}
