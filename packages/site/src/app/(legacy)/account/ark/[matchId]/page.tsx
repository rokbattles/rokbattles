import { ChevronLeftIcon } from "@heroicons/react/16/solid";
import { redirect } from "next/navigation";
import { getExtracted } from "next-intl/server";
import { ArkMatchDetailContent } from "@/components/account-ark/ark-match-detail-content";
import { Heading } from "@/components/ui/heading";
import { Link } from "@/components/ui/link";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page({ params }: PageProps<"/account/ark/[matchId]">) {
  const user = await requireCurrentUserWithGovernor();
  const t = await getExtracted();
  const { matchId } = await params;
  const governorId = user.claimedGovernors[0]?.governorId;

  if (governorId == null) {
    redirect("/account/settings/governors");
  }

  return (
    <>
      <div className="mb-8">
        <Link
          href="/account/ark"
          className="inline-flex items-center gap-2 text-sm/6 text-zinc-400"
        >
          <ChevronLeftIcon aria-hidden="true" className="size-4 fill-zinc-500" />
          {t("Back to Ark Recap")}
        </Link>
      </div>
      <Heading>{t("Ark Match")}</Heading>
      <ArkMatchDetailContent matchId={matchId} />
    </>
  );
}
