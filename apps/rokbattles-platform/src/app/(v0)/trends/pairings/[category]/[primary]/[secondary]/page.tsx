import { ChevronLeftIcon } from "@heroicons/react/16/solid";
import PairingAccessoryDetails from "@/components/trends/PairingAccessoryDetails";
import { Link } from "@/components/ui/Link";
import { Text } from "@/components/ui/Text";

export default async function Page({
  params,
}: PageProps<"/trends/pairings/[category]/[primary]/[secondary]">) {
  const { category, primary, secondary } = await params;
  const primaryId = Number(primary);
  const secondaryId = Number(secondary);

  if (!Number.isFinite(primaryId) || !Number.isFinite(secondaryId)) {
    return <Text className="mt-6 text-sm text-zinc-500">Invalid pairing.</Text>;
  }

  return (
    <>
      <div className="max-lg:hidden mb-8">
        <Link
          href="/trends"
          className="inline-flex items-center gap-2 text-sm/6 text-zinc-500 dark:text-zinc-400"
        >
          <ChevronLeftIcon className="size-4 fill-zinc-400 dark:fill-zinc-500" />
          Explore Trends
        </Link>
      </div>
      <PairingAccessoryDetails
        category={category ?? ""}
        primaryId={primaryId}
        secondaryId={secondaryId}
      />
    </>
  );
}
