import { ChevronLeft } from "lucide-react";
import { getExtracted } from "next-intl/server";
import DuelReportView from "@/components/olympian-arena/duel-report-view";
import { Link } from "@/components/ui/link";

export default async function Page({ params }: PageProps<"/olympian-arena/[id]">) {
  const t = await getExtracted();
  const { id } = await params;

  return (
    <>
      <div className="mb-8 max-lg:hidden">
        <Link
          href="/olympian-arena"
          className="inline-flex items-center gap-2 text-sm/6 text-zinc-500 dark:text-zinc-400"
        >
          <ChevronLeft
            data-slot="icon"
            aria-hidden="true"
            className="size-4 text-zinc-400 dark:text-zinc-500"
          />
          {t("Explore Duels")}
        </Link>
      </div>
      <DuelReportView duelId={id ?? ""} />
    </>
  );
}
