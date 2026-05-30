import { cookies } from "next/headers";
import type { CurrentUser } from "@/lib/types/current-user";

type CurrentUserResponse = {
  user: CurrentUser;
};

export async function getCurrentUser(): Promise<CurrentUser | null> {
  const cookieStore = await cookies();
  const sid = cookieStore.get("sid")?.value;
  if (!sid) {
    return null;
  }

  const response = await fetch(`${process.env.API_URL || "http://localhost:8001"}/v1/auth/me`, {
    headers: {
      Cookie: `sid=${sid}`,
    },
    cache: "no-store",
  });

  if (response.status === 401) {
    return null;
  }

  if (!response.ok) {
    console.error("Failed to fetch current user", response.status);
    return null;
  }

  const payload = (await response.json()) as CurrentUserResponse;
  return payload.user ?? null;
}
