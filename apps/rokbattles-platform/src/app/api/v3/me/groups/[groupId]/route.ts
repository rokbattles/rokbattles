import { notImplementedResponse } from "@/lib/api/not-implemented";

export function GET() {
  /**
   * Fetch details for a single group visible to the current user.
   */
  return notImplementedResponse();
}

export function PATCH() {
  /**
   * Update group metadata (name, settings, etc).
   *
   * Should require admin/owner role.
   */
  return notImplementedResponse();
}

export function DELETE() {
  /**
   * Delete a group or leave it, depending on caller role.
   *
   * If owner: delete group and related ACL data.
   * If member: remove membership.
   */
  return notImplementedResponse();
}
