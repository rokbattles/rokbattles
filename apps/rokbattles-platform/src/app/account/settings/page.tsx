import { redirect } from "next/navigation";
import AccountSettingsContent from "@/components/account/account-settings-content";
import { fetchCurrentUser } from "@/data/fetch-current-user";
import { fetchCurrentUserBinds } from "@/data/fetch-current-user-binds";

export default async function Page() {
  const currentUser = await fetchCurrentUser();
  if (!currentUser) {
    redirect("/api/auth/login");
  }

  const binds = await fetchCurrentUserBinds(currentUser.discordId);

  return <AccountSettingsContent binds={binds} email={currentUser.email} />;
}
