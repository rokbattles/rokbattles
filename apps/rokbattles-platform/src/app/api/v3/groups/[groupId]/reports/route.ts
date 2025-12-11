import { notImplementedResponse } from "@/lib/api/not-implemented";

export function GET() {
  /**
   * List reports visible to a group.
   *
   * Intended to return the same shape as /api/v3/reports, but
   * filtered to reports shared with groupId.
   */
  return notImplementedResponse();
}
