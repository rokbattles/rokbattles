import { notImplementedResponse } from "@/lib/api/not-implemented";

export function GET() {
  /**
   * List the current user's claimed App UIDs and their linked governors.
   *
   * V3 shape: claims are keyed by appUid (string) and include all governors
   * we can associate to that UID, plus refresh metadata for the UI.
   *
   * Intended response (roughly):
   * {
   *   "<appUid>": {
   *     governors: [{ id, name, avatarUrl, frameUrl }],
   *     lastRefresh: <timestamp>, // claim `updatedAt`
   *     nextRefresh: <timestamp>, // `updatedAt` + 1h
   *   },
   * }
   *
   * This replaces the v2 "claimed governor" listing which was per-governorId.
   */

  return notImplementedResponse();
}

export function POST() {
  /**
   * Claim a new App UID for the current user, given a governorId.
   *
   * Replaces /api/v2/governor/claim, but claims at the App UID level.
   *
   * Intended process:
   * - Validate request body includes a governorId.
   * - Enforce max claims (3 App UIDs per user).
   * - Look up appUid for the governorId from battleReports.
   * - Reject if the App UID is already claimed by someone else.
   * - Create/assign the claim to this user.
   * - Auto-link all governors associated to the App UID, populating name/avatar/frame.
   *
   * Notes:
   * - appUid is derived from battle reports.
   * - Governors for an App UID can be aggregated from battleReports.
   * - Latest metadata (name/avatar/frame) can also be pulled from reports.
   *
   * Storage:
   * - Reuse claimedGovernors collection but migrate to App UID keyed docs.
   * - Migration rule: if two users claim governors under the same App UID, oldest claim wins; others are removed.
   *
   * New schema (approx):
   * { discordId, appUid, governors: [{ id, name, avatarUrl, frameUrl }], createdAt, updatedAt }
   */

  return notImplementedResponse();
}
