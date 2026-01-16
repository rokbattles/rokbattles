import { authenticateRequest } from "@/lib/auth";
import type { ClaimedGovernorDocument } from "@/lib/types/auth";
import type { CurrentUser } from "@/lib/types/current-user";

export async function getCurrentUser(): Promise<CurrentUser | null> {
  const authResult = await authenticateRequest();
  if (authResult.ok === false) {
    return null;
  }

  const { db, user } = authResult.context;

  const claimedGovernorsDocs = await db
    .collection<ClaimedGovernorDocument>("claimedGovernors")
    .find(
      { discordId: user.discordId },
      {
        projection: {
          _id: 0,
          governorId: 1,
          governorName: 1,
          governorAvatar: 1,
          createdAt: 1,
        },
      }
    )
    .sort({ createdAt: -1 })
    .toArray();

  const claimedGovernors =
    claimedGovernorsDocs.length > 0
      ? claimedGovernorsDocs.map((claim) => ({
          governorId: claim.governorId,
          governorName: claim.governorName ?? null,
          governorAvatar: claim.governorAvatar ?? null,
        }))
      : [];

  return {
    username: user.username,
    discriminator: user.discriminator,
    globalName: user.globalName ?? null,
    email: user.email,
    avatar: user.avatar ?? null,
    claimedGovernors,
  };
}
