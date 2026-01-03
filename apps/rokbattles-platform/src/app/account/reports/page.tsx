import { AccountReportsContent } from "@/components/account/AccountReportsContent";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  await requireCurrentUserWithGovernor();
  return <AccountReportsContent />;
}
