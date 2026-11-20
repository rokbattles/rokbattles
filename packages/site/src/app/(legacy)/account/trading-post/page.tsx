import { Suspense } from "react";
import { AccountCarriageTransfers } from "@/components/account-trading-post/account-carriage-transfers";
import { Heading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

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
