import { ChevronLeft } from "lucide-react";
import { redirect } from "next/navigation";

import { ArkMatchDetailContent } from "@/components/account-ark/ark-match-detail-content";
import { Heading } from "@/components/ui/heading";
import { Link } from "@/components/ui/link";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page({ params }: PageProps<"/account/ark/[matchId]">) {
  const user = await requireCurrentUserWithGovernor();

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
          <ChevronLeft data-slot="icon" aria-hidden="true" className="size-4 text-zinc-500" />
          Back to Ark Recap
        </Link>
      </div>
      <Heading>Ark Match</Heading>
      <ArkMatchDetailContent matchId={matchId} />
    </>
  );
}
