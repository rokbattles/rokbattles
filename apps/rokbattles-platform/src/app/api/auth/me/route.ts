import { NextResponse } from "next/server";
import { authenticateRequest } from "@/lib/auth";
import type { ClaimedGovernorDocument } from "@/lib/types/auth";

export async function GET() {
  const authResult = await authenticateRequest();
  if (authResult.ok === false) {
    return NextResponse.json({ user: null }, { status: 401 });
  }

  const { db, user } = authResult.context;

  const claimedGovernorsDocs = await db
    .collection<ClaimedGovernorDocument>("claimedGovernors")
    .find(
      { discordId: user.discordId },
      { projection: { _id: 0, governorId: 1, governorName: 1, governorAvatar: 1, createdAt: 1 } }
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

  return NextResponse.json({
    user: {
      username: user.username,
      discriminator: user.discriminator,
      globalName: user.globalName ?? null,
      email: user.email,
      avatar: user.avatar ?? null,
      claimedGovernors,
    },
  });
}
