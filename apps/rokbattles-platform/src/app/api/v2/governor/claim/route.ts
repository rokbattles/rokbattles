import { type NextRequest, NextResponse } from "next/server";
import { requireAuthContext } from "@/lib/auth";
import { parseGovernorId } from "@/lib/governor";
import type { ClaimedGovernorDocument } from "@/lib/types/auth";

interface RawParticipant {
  player_id?: number;
  player_name?: string;
  avatar_url?: string;
}

export async function POST(req: NextRequest) {
  let payload: unknown;
  try {
    payload = await req.json();
  } catch {
    return NextResponse.json({ error: "Invalid JSON body" }, { status: 400 });
  }

  const governorId = parseGovernorId(
    payload && typeof payload === "object"
      ? (payload as Record<string, unknown>).governorId
      : null
  );

  if (governorId == null) {
    return NextResponse.json({ error: "Invalid governorId" }, { status: 400 });
  }

  const authResult = await requireAuthContext();
  if (!authResult.ok) {
    return authResult.response;
  }

  const { db, user } = authResult.context;

  const existingClaim = await db
    .collection<ClaimedGovernorDocument>("claimedGovernors")
    .findOne({ governorId });
  if (existingClaim) {
    return NextResponse.json(
      { error: "Governor already claimed" },
      { status: 409 }
    );
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
        $or: [
          { "report.self.player_id": governorId },
          { "report.enemy.player_id": governorId },
        ],
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
    participant && typeof participant.player_name === "string"
      ? participant.player_name
      : null;
  const governorAvatar =
    participant && typeof participant.avatar_url === "string"
      ? participant.avatar_url
      : null;

  const createdAt = new Date();
  const claim: ClaimedGovernorDocument = {
    discordId: user.discordId,
    governorId,
    createdAt,
    governorName,
    governorAvatar,
  };

  await db
    .collection<ClaimedGovernorDocument>("claimedGovernors")
    .insertOne(claim);

  return NextResponse.json({
    claim,
  });
}
