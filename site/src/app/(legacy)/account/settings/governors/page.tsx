import { AccountBindsContent } from "@/components/account-binds/account-binds-content";
import { requireCurrentUser } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUser();
  return <AccountBindsContent initialUser={user} />;
}
