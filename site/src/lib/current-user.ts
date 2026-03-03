import { cookies } from "next/headers";
import type { CurrentUser } from "@/lib/types/current-user";

type CurrentUserResponse = {
  user: CurrentUser;
};

const SESSION_COOKIE_NAME = "_rokb_session";

export async function getCurrentUser(): Promise<CurrentUser | null> {
  const cookieStore = await cookies();
  const sessionId = cookieStore.get(SESSION_COOKIE_NAME)?.value;
  if (!sessionId) {
    return null;
  }

  const response = await fetch(`${process.env.API_URL || "http://localhost:8001"}/v1/auth/me`, {
    headers: {
      Cookie: `${SESSION_COOKIE_NAME}=${sessionId}`,
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
