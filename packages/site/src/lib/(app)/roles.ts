import type { CurrentUser, UserRole } from "@/lib/types/current-user";

export function hasRole(
  user: Pick<CurrentUser, "roles"> | null | undefined,
  role: UserRole
): boolean {
  return user?.roles.includes(role) ?? false;
}
