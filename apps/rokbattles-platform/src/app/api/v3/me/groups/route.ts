import { notImplementedResponse } from "@/lib/api/not-implemented";

export function GET() {
  /**
   * List groups the current user belongs to.
   *
   * Intended to back a "my groups" picker in the UI.
   */
  return notImplementedResponse();
}

export function POST() {
  /**
   * Create a new group owned by the current user.
   *
   * Expected inputs: name/metadata; current user becomes owner/admin.
   */
  return notImplementedResponse();
}
