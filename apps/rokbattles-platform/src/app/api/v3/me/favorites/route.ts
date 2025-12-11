import { notImplementedResponse } from "@/lib/api/not-implemented";

export function GET() {
  /**
   * List the current user's favorite report parent hashes.
   *
   * Likely used for a "saved battles" view; should return minimal
   * report metadata for display.
   */
  return notImplementedResponse();
}

export function POST() {
  /**
   * Add a report (by parent hash) to the current user's favorites.
   *
   * Should validate the hash exists and avoid duplicates.
   */
  return notImplementedResponse();
}
