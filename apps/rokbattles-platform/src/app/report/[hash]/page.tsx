import { ChevronLeftIcon } from "@heroicons/react/16/solid";
import { ReportView } from "@/components/report/ReportView";
import { Link } from "@/components/ui/Link";

export default async function Page({ params }: PageProps<"/report/[hash]">) {
  const { hash } = await params;

  return (
    <>
      <div className="max-lg:hidden mb-8">
        <Link
          href="/"
          className="inline-flex items-center gap-2 text-sm/6 text-zinc-500 dark:text-zinc-400"
        >
          <ChevronLeftIcon className="size-4 fill-zinc-400 dark:fill-zinc-500" />
          Explore Battles
        </Link>
      </div>
      <ReportView hash={hash ?? ""} />
    </>
  );
}
