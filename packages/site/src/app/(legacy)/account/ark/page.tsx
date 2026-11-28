import { ArkMatchHistoryContent } from "@/components/account-ark/ark-match-history-content";
import { Heading } from "@/components/ui/heading";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default async function Page() {
  await requireCurrentUserWithGovernor();

  return (
    <>
      <Heading>Ark Recap</Heading>
      <ArkMatchHistoryContent />
    </>
  );
}
