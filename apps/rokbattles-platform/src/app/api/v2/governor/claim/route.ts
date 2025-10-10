import { cookies } from "next/headers";
import { type NextRequest, NextResponse } from "next/server";
import client from "@/lib/mongo";

interface SessionDocument {
  sessionId: string;
  userId: string;
  createdAt: Date;
  expiresAt: Date;
}

interface UserDocument {
  discordId: string;
}

interface ClaimedGovernorDocument {
  discordId: string;
  governorId: number;
  createdAt: Date;
  governorName: string | null;
  governorAvatar: string | null;
}

interface RawParticipant {
  player_id?: number;
  player_name?: string;
  avatar_url?: string;
}

function parseGovernorId(value: unknown): number | null {
  if (typeof value === "number" && Number.isFinite(value)) {
    const truncated = Math.trunc(value);
    return truncated > 0 ? truncated : null;
  }

  if (typeof value === "string" && value.trim() !== "") {
    const parsed = Number.parseInt(value, 10);
    return Number.isFinite(parsed) && parsed > 0 ? parsed : null;
  }

  return null;
}

export async function POST(req: NextRequest) {
  const cookieStore = await cookies();

  if (!cookieStore.has("sid")) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const sid = cookieStore.get("sid")?.value;
  const mongo = await client.connect();
  const db = mongo.db();

  const session = await db.collection<SessionDocument>("userSessions").findOne({ sessionId: sid });
  if (!session) {
    cookieStore.delete("sid");
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const now = new Date();
  if (session.expiresAt <= now) {
    await db.collection<SessionDocument>("userSessions").deleteOne({ sessionId: sid });
    cookieStore.delete("sid");
    return NextResponse.json({ error: "Session expired" }, { status: 401 });
  }

  const user = await db.collection<UserDocument>("users").findOne({ discordId: session.userId });
  if (!user) {
    await db.collection<SessionDocument>("userSessions").deleteOne({ sessionId: sid });
    cookieStore.delete("sid");
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  let payload: unknown;
  try {
    payload = await req.json();
  } catch {
    return NextResponse.json({ error: "Invalid JSON body" }, { status: 400 });
  }

  const governorId = parseGovernorId(
    payload && typeof payload === "object" ? (payload as Record<string, unknown>).governorId : null
  );

  if (!governorId) {
    return NextResponse.json({ error: "Invalid governorId" }, { status: 400 });
  }

  const existingClaim = await db
    .collection<ClaimedGovernorDocument>("claimedGovernors")
    .findOne({ governorId });
  if (existingClaim) {
    if (existingClaim.discordId === user.discordId) {
      return NextResponse.json(
        {
          claim: {
            discordId: existingClaim.discordId,
            governorId: existingClaim.governorId,
            createdAt: existingClaim.createdAt,
            governorName: existingClaim.governorName,
            governorAvatar: existingClaim.governorAvatar,
            alreadyClaimed: true,
          },
        },
        { status: 200 }
      );
    }

    return NextResponse.json({ error: "Governor already claimed" }, { status: 409 });
  }

  const currentClaims = await db
    .collection<ClaimedGovernorDocument>("claimedGovernors")
    .countDocuments({ discordId: user.discordId });
  if (currentClaims >= 3) {
    return NextResponse.json({ error: "Claim limit reached" }, { status: 409 });
  }

  const latestReport = await db
    .collection("battleReports")
    .find(
      {
        $or: [{ "report.self.player_id": governorId }, { "report.enemy.player_id": governorId }],
      },
      {
        projection: {
          "report.self.player_id": 1,
          "report.self.player_name": 1,
          "report.self.avatar_url": 1,
          "report.enemy.player_id": 1,
          "report.enemy.player_name": 1,
          "report.enemy.avatar_url": 1,
          "report.metadata.email_time": 1,
        },
      }
    )
    .sort({ "report.metadata.email_time": -1 })
    .limit(1)
    .next();

  let participant: RawParticipant | null = null;
  if (latestReport?.report && typeof latestReport.report === "object") {
    const report = latestReport.report as {
      self?: RawParticipant;
      enemy?: RawParticipant;
    };

    if (report.self?.player_id === governorId) {
      participant = report.self;
    } else if (report.enemy?.player_id === governorId) {
      participant = report.enemy;
    }
  }

  const governorName =
    participant && typeof participant.player_name === "string" ? participant.player_name : null;
  const governorAvatar =
    participant && typeof participant.avatar_url === "string" ? participant.avatar_url : null;

  const createdAt = new Date();
  const claim: ClaimedGovernorDocument = {
    discordId: user.discordId,
    governorId,
    createdAt,
    governorName,
    governorAvatar,
  };

  await db.collection<ClaimedGovernorDocument>("claimedGovernors").insertOne(claim);

  return NextResponse.json({
    claim,
  });
}
