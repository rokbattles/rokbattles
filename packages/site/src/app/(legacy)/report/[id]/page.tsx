import { ChevronLeftIcon } from "@heroicons/react/16/solid";
import type { Metadata } from "next";
import { getExtracted } from "next-intl/server";
import { ReportView } from "@/components/report/report-view";
import { Link } from "@/components/ui/link";

export async function generateMetadata(): Promise<Metadata> {
  const t = await getExtracted();
  const title = t("Battle Report");

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
  const t = await getExtracted();
  const { id } = await params;
  const resolvedSearchParams = (await searchParams) ?? {};
  const fromParam = resolveSearchParam(resolvedSearchParams.from);
  const isAccountReports = fromParam === "account-reports" || fromParam === "my-reports";
  const arkMatchId =
    fromParam === "ark" ? resolveSearchParam(resolvedSearchParams.matchId) : undefined;
  const backBase = isAccountReports ? "/account/reports" : "/";
  const backLabel = isAccountReports ? t("Back to My Reports") : t("Explore Battles");
  const backQuery = buildQueryString(resolvedSearchParams, "from");
  const backHref = arkMatchId
    ? `/account/ark/${encodeURIComponent(arkMatchId)}?tab=reports`
    : `${backBase}${backQuery}`;

  return (
    <>
      <div className={arkMatchId ? "mb-8" : "max-lg:hidden mb-8"}>
        <Link
          href={backHref}
          className="inline-flex items-center gap-2 text-sm/6 text-zinc-500 dark:text-zinc-400"
        >
          <ChevronLeftIcon className="size-4 fill-zinc-400 dark:fill-zinc-500" />
          {arkMatchId ? t("Back to Ark Recap") : backLabel}
        </Link>
      </div>
      <ReportView id={id ?? ""} />
    </>
  );
}
