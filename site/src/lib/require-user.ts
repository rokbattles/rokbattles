import { redirect } from "next/navigation";
import { getCurrentUser } from "@/lib/current-user";
import type { CurrentUser } from "@/lib/types/current-user";

export async function requireCurrentUser(): Promise<CurrentUser> {
  const user = await getCurrentUser();
  if (!user) {
    redirect("/");
  }

  return user;
}

export async function requireCurrentUserWithGovernor(): Promise<CurrentUser> {
  const user = await requireCurrentUser();
  if (!user.claimedGovernors || user.claimedGovernors.length === 0) {
    redirect("/account/settings/governors");
  }

  return user;
}
