import { cookies } from "next/headers";
import { cache } from "react";
import type { AppPermission } from "@/lib/(app)/developer-apps";

export type AuthorizedApp = {
  id: string;
  name: string;
  authorizedOn: number;
  termsUrl: string | null;
  privacyUrl: string | null;
  permissions: AppPermission[];
};

export const getAuthorizedApps = cache(async (): Promise<AuthorizedApp[] | null> => {
  const sid = (await cookies()).get("sid")?.value;
  if (!sid) {
    return null;
  }

  const api = process.env.API_URL || "http://localhost:8001";
  const response = await fetch(`${api}/v1/auth/authorized-apps`, {
    headers: { Cookie: `sid=${sid}` },
    cache: "no-store",
  });

  return response.ok ? ((await response.json()) as AuthorizedApp[]) : null;
});
