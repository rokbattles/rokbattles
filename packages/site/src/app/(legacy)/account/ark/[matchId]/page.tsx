import { ArkMatchDetailContent } from "@/components/account-ark/ark-match-detail-content";
import { Heading } from "@/components/ui/heading";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page({ params }: PageProps<"/account/ark/[matchId]">) {
  await requireCurrentUserWithGovernor();

  const { matchId } = await params;

  return (
    <>
      <Heading>Ark Match</Heading>
      <ArkMatchDetailContent matchId={matchId} />
    </>
  );
}
