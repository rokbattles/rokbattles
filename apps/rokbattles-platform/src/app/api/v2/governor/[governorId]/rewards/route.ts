import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import { requireAuthContext } from "@/lib/auth";
import { normalizeTimestampMillis } from "@/lib/datetime";
import { parseGovernorId } from "@/lib/governor";
import { resolveDateRange } from "@/lib/pairings/shared";
import type { ClaimedGovernorDocument } from "@/lib/types/auth";

type NpcRewardEntry = {
  type?: unknown;
  sub_type?: unknown;
  value?: unknown;
};

type BattleReportDocument = {
  report?: {
    metadata?: {
      email_time?: unknown;
    };
    self?: {
      player_id?: unknown;
    };
    enemy?: {
      player_id?: unknown;
      npc_type?: unknown;
      npc_btype?: unknown;
      npc_rewards?: NpcRewardEntry[] | null;
    };
  };
};

type RewardBucket = {
  type: number;
  subType: number;
  total: number;
  count: number;
};

function extractEventTimeMillis(report: BattleReportDocument["report"]): number | null {
  const emailTime = normalizeTimestampMillis(report?.metadata?.email_time);
  if (emailTime != null) {
    return emailTime;
  }

  return null;
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
  const startMicros = Math.floor(startMillis * 1000);
  const endMicros = Math.floor(endMillis * 1000);

  return {
    $and: [
      { "report.self.player_id": governorId },
      { "report.enemy.player_id": -2 },
      {
        "report.metadata.email_time": { $gte: startMicros, $lt: endMicros },
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
    "report.metadata.email_time": 1,
    "report.enemy.player_id": 1,
    "report.enemy.npc_type": 1,
    "report.enemy.npc_btype": 1,
    "report.enemy.npc_rewards": 1,
  };

  try {
    const reports = await db
      .collection<BattleReportDocument>("battleReports")
      .find(buildMatchStage(governorId, range.startMillis, range.endMillis), { projection })
      .toArray();

    const rewardBuckets = new Map<string, RewardBucket>();

    let totalReports = 0;
    let barbKills = 0;
    let barbFortKills = 0;
    let otherNpcKills = 0;

    for (const doc of reports) {
      const report = doc.report;
      if (!report) {
        continue;
      }

      const eventTime = extractEventTimeMillis(report);
      if (eventTime == null || eventTime < range.startMillis || eventTime >= range.endMillis) {
        continue;
      }

      const npcBtype = parseNumeric(report.enemy?.npc_btype);
      const npcType = parseNumeric(report.enemy?.npc_type);
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

      const rewards = report.enemy?.npc_rewards;
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
