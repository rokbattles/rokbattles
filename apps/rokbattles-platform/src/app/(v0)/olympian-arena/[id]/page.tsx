import { ChevronLeftIcon } from "@heroicons/react/16/solid";
import DuelReportView from "@/components/olympian-arena/DuelReportView";
import { Link } from "@/components/ui/Link";

export default async function Page({ params }: PageProps<"/olympian-arena/[id]">) {
  const { id } = await params;

  return (
    <>
      <div className="mb-8 max-lg:hidden">
        <Link
          href="/olympian-arena"
          className="inline-flex items-center gap-2 text-sm/6 text-zinc-500 dark:text-zinc-400"
        >
          <ChevronLeftIcon className="size-4 fill-zinc-400 dark:fill-zinc-500" />
          Explore Duels
        </Link>
      </div>
      <DuelReportView duelId={id ?? ""} />
    </>
  );
}
