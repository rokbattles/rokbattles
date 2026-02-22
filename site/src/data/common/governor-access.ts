import "server-only";

import { requireAuthContext } from "@/lib/auth";
import type { ClaimedGovernorDocument } from "@/lib/types/auth";

export class GovernorAccessError extends Error {
  readonly status: number;
  readonly reason: string;

  constructor(status: number, message: string, reason: string) {
    super(message);
    this.status = status;
    this.reason = reason;
  }
}

export async function requireGovernorAccess(governorId: number) {
  const authResult = await requireAuthContext();
  if (!authResult.ok) {
    throw new GovernorAccessError(401, "Unauthorized", authResult.reason);
  }

  const { db, user } = authResult.context;
  const claim = await db
    .collection<ClaimedGovernorDocument>("claimedGovernors")
    .findOne({ discordId: user.discordId, governorId }, { projection: { _id: 0, governorId: 1 } });

  if (!claim) {
    throw new GovernorAccessError(403, "Forbidden", "governor-not-claimed");
  }

  return authResult.context;
}
