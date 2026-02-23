import { type NextRequest, NextResponse } from "next/server";
import { requireAuthContext } from "@/lib/auth";
import { parseGovernorId } from "@/lib/governor";
import type { ClaimedGovernorDocument } from "@/lib/types/auth";

interface RawParticipant {
  player_id?: number;
  player_name?: string;
  avatar_url?: string;
}

interface RawBattleMail {
  sender?: RawParticipant;
  opponents?: RawParticipant[];
}

function extractParticipant(mail: RawBattleMail | null, governorId: number): RawParticipant | null {
  if (!mail || typeof mail !== "object") {
    return null;
  }

  if (mail.sender?.player_id === governorId) {
    return mail.sender;
  }

  if (!Array.isArray(mail.opponents)) {
    return null;
  }

  for (const opponent of mail.opponents) {
    if (opponent?.player_id === governorId) {
      return opponent;
    }
  }

  return null;
}

export async function POST(req: NextRequest) {
  let payload: unknown;
  try {
    payload = await req.json();
  } catch {
    return NextResponse.json({ error: "Invalid JSON body" }, { status: 400 });
  }

  const governorId = parseGovernorId(
    payload && typeof payload === "object" ? (payload as Record<string, unknown>).governorId : null
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
    return NextResponse.json({ error: "Governor already claimed" }, { status: 409 });
  }

  const currentClaims = await db
    .collection<ClaimedGovernorDocument>("claimedGovernors")
    .countDocuments({ discordId: user.discordId });
  if (currentClaims >= 3) {
    return NextResponse.json({ error: "Claim limit reached" }, { status: 409 });
  }

  const latestMail = await db
    .collection<RawBattleMail>("mails_battle")
    .find(
      {
        $or: [{ "sender.player_id": governorId }, { "opponents.player_id": governorId }],
      },
      {
        projection: {
          "sender.player_id": 1,
          "sender.player_name": 1,
          "sender.avatar_url": 1,
          "opponents.player_id": 1,
          "opponents.player_name": 1,
          "opponents.avatar_url": 1,
          "metadata.mail_time": 1,
        },
      }
    )
    .sort({ "metadata.mail_time": -1 })
    .limit(1)
    .next();

  const participant = extractParticipant(latestMail, governorId);

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
