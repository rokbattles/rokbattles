import { notImplementedResponse } from "@/lib/api/not-implemented";

export function POST() {
  /**
   * Refresh a claimed App UID's governor metadata.
   *
   * Intended process:
   * - Verify the current user owns the App UID.
   * - For each linked governor:
   *   - Pull latest name/avatar/frame from recent battleReports.
   * - Discover any new governorIds seen for this App UID and link them.
   *
   * Effectively the "re-sync" step for /api/v3/me/governors.
   */

  return notImplementedResponse();
}
