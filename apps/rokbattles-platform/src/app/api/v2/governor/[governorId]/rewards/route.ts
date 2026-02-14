import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import { requireAuthContext } from "@/lib/auth";
import { normalizeTimestampMillis } from "@/lib/datetime";
import { parseGovernorId } from "@/lib/governor";
import { resolveDateRange } from "@/lib/pairings/shared";
import type { ClaimedGovernorDocument } from "@/lib/types/auth";

type BattleLootEntry = {
  type?: unknown;
  sub_type?: unknown;
  value?: unknown;
};

type BattleOpponentDocument = {
  player_id?: unknown;
  npc?: {
    type?: unknown;
    b_type?: unknown;
    loot?: BattleLootEntry[] | null;
  };
};

type BattleMailDocument = {
  metadata?: {
    mail_time?: unknown;
  };
  sender?: {
    player_id?: unknown;
  };
  opponents?: BattleOpponentDocument[] | null;
};

type RewardBucket = {
  type: number;
  subType: number;
  total: number;
  count: number;
};

function extractEventTimeMillis(mail: BattleMailDocument): number | null {
  return normalizeTimestampMillis(mail.metadata?.mail_time);
}

function parseNumeric(value: unknown): number | null {
  if (value == null) {
    return null;
  }

  const numeric = typeof value === "bigint" ? Number(value) : Number(value);
  return Number.isFinite(numeric) ? numeric : null;
}

function isKvkBarbarian(npcType: number | null, npcBtype: number | null) {
  if (npcType == null || npcBtype == null) {
    return false;
  }

  return npcBtype === 1 && npcType >= 401 && npcType <= 415;
}

function isKvkBarbarianFort(npcType: number | null, npcBtype: number | null) {
  if (npcType == null || npcBtype == null) {
    return false;
  }

  return npcBtype === 2 && npcType >= 121 && npcType <= 125;
}

function buildMatchStage(governorId: number, startMillis: number, endMillis: number): Document {
  const startSeconds = Math.floor(startMillis / 1000);
  const endSeconds = Math.floor(endMillis / 1000);
  const startMicros = Math.floor(startMillis * 1000);
  const endMicros = Math.floor(endMillis * 1000);

  return {
    $and: [
      { "sender.player_id": governorId },
      { opponents: { $elemMatch: { player_id: -2 } } },
      {
        $or: [
          { "metadata.mail_time": { $gte: startSeconds, $lt: endSeconds } },
          { "metadata.mail_time": { $gte: startMillis, $lt: endMillis } },
          { "metadata.mail_time": { $gte: startMicros, $lt: endMicros } },
        ],
      },
    ],
  };
}

export async function GET(
  req: NextRequest,
  ctx: RouteContext<"/api/v2/governor/[governorId]/rewards">
) {
  const { governorId: governorParam } = await ctx.params;
  const governorId = parseGovernorId(governorParam);
  if (governorId == null) {
    return NextResponse.json({ error: "Invalid governorId" }, { status: 400 });
  }

  const authResult = await requireAuthContext();
  if (!authResult.ok) {
    return authResult.response;
  }

  const { db, user } = authResult.context;
  const nowYear = new Date().getUTCFullYear();
  const url = new URL(req.url);
  const yearParam = url.searchParams.get("year");
  const startParam = url.searchParams.get("start");
  const endParam = url.searchParams.get("end");
  const parsedYear = yearParam ? Number(yearParam) : Number.NaN;
  const targetYear = Number.isFinite(parsedYear) ? parsedYear : nowYear;
  const range = resolveDateRange({ startParam, endParam, fallbackYear: targetYear });

  const claim = await db
    .collection<ClaimedGovernorDocument>("claimedGovernors")
    .findOne({ discordId: user.discordId, governorId });
  if (!claim) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const projection: Document = {
    _id: 0,
    "metadata.mail_time": 1,
    "opponents.player_id": 1,
    "opponents.npc.type": 1,
    "opponents.npc.b_type": 1,
    "opponents.npc.loot": 1,
  };

  try {
    const mails = await db
      .collection<BattleMailDocument>("mails_battle")
      .find(buildMatchStage(governorId, range.startMillis, range.endMillis), { projection })
      .toArray();

    const rewardBuckets = new Map<string, RewardBucket>();

    let totalReports = 0;
    let barbKills = 0;
    let barbFortKills = 0;
    let otherNpcKills = 0;

    for (const mail of mails) {
      const eventTime = extractEventTimeMillis(mail);
      if (eventTime == null || eventTime < range.startMillis || eventTime >= range.endMillis) {
        continue;
      }

      const opponents = mail.opponents;
      if (!Array.isArray(opponents) || opponents.length === 0) {
        continue;
      }

      for (const opponent of opponents) {
        const opponentId = parseNumeric(opponent.player_id);
        if (opponentId !== -2) {
          continue;
        }

        const npcBtype = parseNumeric(opponent.npc?.b_type);
        const npcType = parseNumeric(opponent.npc?.type);
        const isKvkBarb = isKvkBarbarian(npcType, npcBtype);
        const isKvkFort = isKvkBarbarianFort(npcType, npcBtype);

        totalReports += 1;

        if (isKvkBarb) {
          barbKills += 1;
        } else if (isKvkFort) {
          barbFortKills += 1;
        } else {
          otherNpcKills += 1;
        }

        if (!isKvkBarb && !isKvkFort) {
          continue;
        }

        const rewards = opponent.npc?.loot;
        if (!Array.isArray(rewards)) {
          continue;
        }

        for (const reward of rewards) {
          const type = parseNumeric(reward.type);
          const subType = parseNumeric(reward.sub_type);
          const value = parseNumeric(reward.value);

          if (type == null || subType == null || value == null) {
            continue;
          }

          const key = `${type}:${subType}`;
          const existing = rewardBuckets.get(key);
          if (existing) {
            existing.total += value;
            existing.count += 1;
          } else {
            rewardBuckets.set(key, {
              type,
              subType,
              total: value,
              count: 1,
            });
          }
        }
      }
    }

    const rewards = Array.from(rewardBuckets.values());

    return NextResponse.json(
      {
        year: range.year,
        stats: {
          totalReports,
          barbKills,
          barbFortKills,
          otherNpcKills,
        },
        rewards,
      },
      {
        headers: {
          "Cache-Control": "no-store",
        },
      }
    );
  } catch (error) {
    console.error("Failed to load governor rewards", error);
    return NextResponse.json(
      { error: "Failed to load governor rewards" },
      {
        status: 500,
        headers: {
          "Cache-Control": "no-store",
        },
      }
    );
  }
}
