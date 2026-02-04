import type { FavoriteReportType } from "@/lib/types/favorite";

export function parseReportType(value: string | null | undefined): FavoriteReportType | null {
  if (!value || value === "battle") {
    return "battle";
  }
  if (value === "duel") {
    return "duel";
  }
  return null;
}
