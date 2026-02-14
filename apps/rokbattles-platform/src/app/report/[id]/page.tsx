import { ChevronLeftIcon } from "@heroicons/react/16/solid";
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { ReportView } from "@/components/report/report-view";
import { Link } from "@/components/ui/link";

export async function generateMetadata(): Promise<Metadata> {
  const t = await getTranslations("report");
  const title = t("battleReportTitle");

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
  const [t, tNav] = await Promise.all([getTranslations("report"), getTranslations("navigation")]);
  const { id } = await params;
  const resolvedSearchParams = (await searchParams) ?? {};
  const fromParam = resolveSearchParam(resolvedSearchParams.from);
  const isAccountReports = fromParam === "account-reports" || fromParam === "my-reports";
  const backBase = isAccountReports ? "/account/reports" : "/";
  const backLabel = isAccountReports ? t("back.reports") : tNav("exploreBattles");
  const backQuery = buildQueryString(resolvedSearchParams, "from");

  return (
    <>
      <div className="max-lg:hidden mb-8">
        <Link
          href={`${backBase}${backQuery}`}
          className="inline-flex items-center gap-2 text-sm/6 text-zinc-500 dark:text-zinc-400"
        >
          <ChevronLeftIcon className="size-4 fill-zinc-400 dark:fill-zinc-500" />
          {backLabel}
        </Link>
      </div>
      <ReportView id={id ?? ""} />
    </>
  );
}
