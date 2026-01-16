import { ChevronLeftIcon } from "@heroicons/react/16/solid";
import { getTranslations } from "next-intl/server";
import DuelReportView from "@/components/olympian-arena/duel-report-view";
import { Link } from "@/components/ui/link";

export default async function Page({
  params,
}: PageProps<"/olympian-arena/[id]">) {
  const t = await getTranslations("navigation");
  const { id } = await params;

  return (
    <>
      <div className="mb-8 max-lg:hidden">
        <Link
          className="inline-flex items-center gap-2 text-sm/6 text-zinc-500 dark:text-zinc-400"
          href="/olympian-arena"
        >
          <ChevronLeftIcon className="size-4 fill-zinc-400 dark:fill-zinc-500" />
          {t("exploreDuels")}
        </Link>
      </div>
      <DuelReportView duelId={id ?? ""} />
    </>
  );
}
