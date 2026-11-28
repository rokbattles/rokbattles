import { ArkMatchDetailContent } from "@/components/account-ark/ark-match-detail-content";
import { Heading } from "@/components/ui/heading";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

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
