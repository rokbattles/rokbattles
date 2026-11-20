"use client";

import { useSearchParams } from "next/navigation";
import { resolveReportsRange } from "@/lib/reports-date-range";

/** The date range for report lists. Defaults to one year back from today, worked out on each load. */
export function useReportsRange() {
  const params = useSearchParams();
  const today = new Date().toISOString().slice(0, 10);
  const range = resolveReportsRange(params.get("start"), params.get("end"), today);

  return { range, today };
}
