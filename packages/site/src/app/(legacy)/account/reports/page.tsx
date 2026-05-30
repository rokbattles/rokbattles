import { AccountReportsContent } from "@/components/account/account-reports-content";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  await requireCurrentUserWithGovernor();
  return <AccountReportsContent />;
}
