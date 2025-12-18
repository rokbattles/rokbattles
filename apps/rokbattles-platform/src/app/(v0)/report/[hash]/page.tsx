import { ChevronLeftIcon } from "@heroicons/react/16/solid";
import type { Metadata } from "next";
import { ReportView } from "@/components/report/ReportView";
import { Link } from "@/components/ui/Link";

export async function generateMetadata({ params }: PageProps<"/report/[hash]">): Promise<Metadata> {
  const { hash } = await params;

  const normalizedHash = hash?.trim() ?? "";
  const imageUrl = `/report/${encodeURIComponent(normalizedHash)}/opengraph-image`;

  return {
    title: "Battle Report",
    openGraph: {
      title: "Battle Report",
      images: [{ url: imageUrl }],
    },
    twitter: {
      card: "summary_large_image",
      title: "Battle Report",
      images: [imageUrl],
    },
  };
}

export default async function Page({ params }: PageProps<"/report/[hash]">) {
  const { hash } = await params;

  return (
    <>
      <div className="max-lg:hidden mb-8">
        <Link
          href="/apps/rokbattles-platform/public"
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
