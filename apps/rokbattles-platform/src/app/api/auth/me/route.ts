import { cookies } from "next/headers";
import { NextResponse } from "next/server";
import client from "@/lib/mongo";

interface SessionDocument {
  sessionId: string;
  userId: string;
  createdAt: Date;
  expiresAt: Date;
}

interface UserDocument {
  discordId: string;
  username: string;
  discriminator: string;
  globalName: string | null;
  email: string;
  avatar: string | null;
  createdAt?: Date;
  updatedAt?: Date;
}

interface ClaimedGovernorDocument {
  discordId: string;
  governorId: number;
  governorName: string | null;
  governorAvatar: string | null;
  createdAt: Date;
}

export async function GET() {
  const cookieStore = await cookies();

  if (!cookieStore.has("sid")) {
    return NextResponse.json({ user: null }, { status: 401 });
  }

  const sid = cookieStore.get("sid").value;
  const mongo = await client.connect();
  const db = mongo.db();

  const session = await db.collection<SessionDocument>("userSessions").findOne({ sessionId: sid });
  if (!session) {
    cookieStore.delete("sid");
    return NextResponse.json({ user: null }, { status: 401 });
  }

  const now = new Date();
  if (session.expiresAt <= now) {
    cookieStore.delete("sid");
    return NextResponse.json({ user: null }, { status: 401 });
  }

  const user = await db.collection<UserDocument>("users").findOne({ discordId: session.userId });
  if (!user) {
    await db.collection<SessionDocument>("userSessions").deleteOne({ sessionId: sid });
    cookieStore.delete("sid");
    return NextResponse.json({ user: null }, { status: 401 });
  }

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
          governorId: Math.trunc(claim.governorId),
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
