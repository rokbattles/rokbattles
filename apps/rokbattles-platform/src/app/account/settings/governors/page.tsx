import { AccountGovernorsContent } from "@/components/account/AccountGovernorsContent";
import { requireCurrentUser } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUser();
  return <AccountGovernorsContent initialUser={user} />;
}
