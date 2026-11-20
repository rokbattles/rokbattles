import { AccountReportsContent } from "@/components/account/account-reports-content";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  await requireCurrentUserWithGovernor();

  // Rallies and garrisons have their own pages, so this list shows open field battles.
  return <AccountReportsContent title="Open Field Battles" kind="openfield" />;
}
