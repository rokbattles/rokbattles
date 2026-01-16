import { ChevronLeftIcon } from "@heroicons/react/16/solid";
import { getTranslations } from "next-intl/server";
import PairingAccessoryDetails from "@/components/trends/pairing-accessory-details";
import { Link } from "@/components/ui/link";
import { Text } from "@/components/ui/text";

export default async function Page({
  params,
}: PageProps<"/trends/pairings/[category]/[primary]/[secondary]">) {
  const [t, tNav] = await Promise.all([
    getTranslations("trends"),
    getTranslations("navigation"),
  ]);
  const { category, primary, secondary } = await params;
  const primaryId = Number(primary);
  const secondaryId = Number(secondary);

  if (!(Number.isFinite(primaryId) && Number.isFinite(secondaryId))) {
    return (
      <Text className="mt-6 text-sm text-zinc-500">
        {t("states.invalidPairing")}
      </Text>
    );
  }

  return (
    <>
      <div className="mb-8 max-lg:hidden">
        <Link
          className="inline-flex items-center gap-2 text-sm/6 text-zinc-500 dark:text-zinc-400"
          href="/trends"
        >
          <ChevronLeftIcon className="size-4 fill-zinc-400 dark:fill-zinc-500" />
          {tNav("exploreTrends")}
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
