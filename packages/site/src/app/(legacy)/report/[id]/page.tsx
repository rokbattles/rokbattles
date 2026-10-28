import { ChevronLeft } from "lucide-react";
import type { Metadata } from "next";

import { ReportView } from "@/components/report/report-view";
import { Link } from "@/components/ui/link";

export async function generateMetadata(): Promise<Metadata> {
  const title = "Battle Report";

  return {
    title,
  };
}

type SearchParams = Record<string, string | string[] | undefined>;

function resolveSearchParam(value: string | string[] | undefined) {
  if (Array.isArray(value)) {
    return value[0];
  }
  return value;
}

function buildQueryString(searchParams: SearchParams, ignoreKey: string) {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(searchParams)) {
    if (key === ignoreKey) {
      continue;
    }
    if (Array.isArray(value)) {
      value.forEach((entry) => {
        if (entry != null) {
          params.append(key, entry);
        }
      });
    } else if (value != null) {
      params.set(key, value);
    }
  }
  const query = params.toString();
  return query ? `?${query}` : "";
}

export default async function Page({ params, searchParams }: PageProps<"/report/[id]">) {
  const { id } = await params;
  const resolvedSearchParams = (await searchParams) ?? {};
  const fromParam = resolveSearchParam(resolvedSearchParams.from);
  const isAccountReports = fromParam === "account-reports" || fromParam === "my-reports";
  const arkMatchId =
    fromParam === "ark" ? resolveSearchParam(resolvedSearchParams.matchId) : undefined;
  const backBase = isAccountReports ? "/account/reports" : "/";
  const backLabel = isAccountReports ? "Back to My Reports" : "Explore Battles";
  const backQuery = buildQueryString(resolvedSearchParams, "from");
  const backHref = arkMatchId
    ? `/account/ark/${encodeURIComponent(arkMatchId)}?tab=reports`
    : `${backBase}${backQuery}`;

  return (
    <>
      <div className={arkMatchId ? "mb-8" : "max-lg:hidden mb-8"}>
        <Link href={backHref} className="inline-flex items-center gap-2 text-sm/6 text-zinc-400">
          <ChevronLeft data-slot="icon" aria-hidden="true" className="size-4 text-zinc-500" />
          {arkMatchId ? "Back to Ark Recap" : backLabel}
        </Link>
      </div>
      <ReportView id={id ?? ""} />
    </>
  );
}
