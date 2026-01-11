import { ChevronLeftIcon } from "@heroicons/react/16/solid";
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { ReportView } from "@/components/report/ReportView";
import { Link } from "@/components/ui/Link";

export async function generateMetadata({ params }: PageProps<"/report/[hash]">): Promise<Metadata> {
  const t = await getTranslations("report");
  const { hash } = await params;

  const normalizedHash = hash?.trim() ?? "";
  const imageUrl = `/report/${encodeURIComponent(normalizedHash)}/opengraph-image`;
  const title = t("battleReportTitle");

  return {
    title,
    openGraph: {
      title,
      images: [{ url: imageUrl }],
    },
    twitter: {
      card: "summary_large_image",
      title,
      images: [imageUrl],
    },
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

export default async function Page({ params, searchParams }: PageProps<"/report/[hash]">) {
  const [t, tNav] = await Promise.all([getTranslations("report"), getTranslations("navigation")]);
  const { hash } = await params;
  const resolvedSearchParams = (await searchParams) ?? {};
  const fromParam = resolveSearchParam(resolvedSearchParams.from);
  const mergeParam = resolveSearchParam(resolvedSearchParams.merge);
  const mergeMode = mergeParam === "1" || mergeParam === "true";
  const isAccountReports = fromParam === "account-reports" || fromParam === "my-reports";
  const isAccountFavorites = fromParam === "account-favorites" || fromParam === "my-favorites";
  const backBase = isAccountReports
    ? "/account/reports"
    : isAccountFavorites
      ? "/account/favorites"
      : "/";
  const backLabel = isAccountReports
    ? t("back.reports")
    : isAccountFavorites
      ? t("back.favorites")
      : tNav("exploreBattles");
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
      <ReportView hash={hash ?? ""} mergeMode={mergeMode} />
    </>
  );
}
