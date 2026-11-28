import { Suspense } from "react";
import { AccountCarriageTransfers } from "@/components/account-trading-post/account-carriage-transfers";
import { Heading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default async function Page() {
  await requireCurrentUserWithGovernor();

  return (
    <div className="space-y-6">
      <Heading>Trading Post</Heading>
      <Suspense fallback={<Text role="status">Loading carriage transfers…</Text>}>
        <AccountCarriageTransfers />
      </Suspense>
    </div>
  );
}
