import type { CurrentUser } from "@/lib/types/current-user";

export function hasOverlayAccess(user: CurrentUser | null | undefined): boolean {
  return user?.roles.includes("content_creator") ?? false;
}
