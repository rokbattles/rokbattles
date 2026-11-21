import type { Metadata } from "next";
import { cookies } from "next/headers";
import { notFound, redirect } from "next/navigation";
import { ProgramApplicationsTable } from "@/components/(app)/admin/program-applications-table";
import { Heading } from "@/components/ui/heading";
import { Pagination, PaginationNext, PaginationPrevious } from "@/components/ui/pagination";
import { Text } from "@/components/ui/text";
import {
  ADMIN_PROGRAM_APPLICATIONS_PATH,
  type AdminApplicationPage,
} from "@/lib/(app)/admin-program-applications";

export const metadata: Metadata = { title: "Program Applications" };

async function getApplications(page: number): Promise<AdminApplicationPage> {
  const sid = (await cookies()).get("sid")?.value;
  if (!sid) redirect("/");

  const origin = process.env.API_URL || "http://localhost:8001";
  const response = await fetch(`${origin}${ADMIN_PROGRAM_APPLICATIONS_PATH}?page=${page}`, {
    headers: { Cookie: `sid=${sid}` },
    cache: "no-store",
  });
  if (response.status === 401) redirect("/");
  if (response.status === 403) notFound();
  if (!response.ok) {
    throw new Error(`Admin program applications request failed (${response.status})`);
  }
  return (await response.json()) as AdminApplicationPage;
}

function parsePage(rawPage: string | string[] | undefined) {
  const page = Number(rawPage);
  return Number.isInteger(page) && page > 0 && page <= 4_294_967_295 ? page : 1;
}

export default async function Page({ searchParams }: PageProps<"/admin/program-applications">) {
  const page = parsePage((await searchParams).page);
  const { applications, hasNextPage } = await getApplications(page);
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
