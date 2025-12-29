import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import { requireAuthContext } from "@/lib/auth";
import { normalizeTimestampMillis } from "@/lib/datetime";
import { parseGovernorId } from "@/lib/governor";
import type { ClaimedGovernorDocument } from "@/lib/types/auth";

type BattleReportDocument = {
  _id?: unknown;
  report?: {
    metadata?: {
      parentHash?: unknown;
      hash?: unknown;
      email_time?: unknown;
      start_date?: unknown;
    };
    self?: { player_id?: unknown };
    enemy?: { player_id?: unknown };
  };
};

type BattleLogDay = {
  date: string;
  battleCount: number;
  npcCount: number;
};

function extractEventTimeMillis(report: BattleReportDocument["report"]): number | null {
  const emailTime = normalizeTimestampMillis(report?.metadata?.email_time);
  if (emailTime != null) {
    return emailTime;
  }

  const startDate = normalizeTimestampMillis(report?.metadata?.start_date);
  if (startDate != null) {
    return startDate;
  }

  return null;
}

function formatDayKey(date: Date) {
  const year = date.getUTCFullYear();
  const month = date.getUTCMonth() + 1;
  const day = date.getUTCDate();

  return `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
}

function buildMatchStage(governorId: number, startMillis: number, endMillis: number): Document {
  const startSeconds = Math.floor(startMillis / 1000);
  const endSeconds = Math.floor(endMillis / 1000);
  const startMicros = Math.floor(startMillis * 1000);
  const endMicros = Math.floor(endMillis * 1000);

  return {
    $and: [
      { "report.enemy.player_id": { $ne: 0 } },
      { $or: [{ "report.self.player_id": governorId }, { "report.enemy.player_id": governorId }] },
      {
        $or: [
          { "report.metadata.start_date": { $gte: startSeconds, $lt: endSeconds } },
          { "report.metadata.email_time": { $gte: startMillis, $lt: endMillis } },
          { "report.metadata.email_time": { $gte: startMicros, $lt: endMicros } },
        ],
      },
    ],
  };
}

export async function GET(
  req: NextRequest,
  ctx: RouteContext<"/api/v2/governor/[governorId]/battle-log">
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

  const claim = await db
    .collection<ClaimedGovernorDocument>("claimedGovernors")
    .findOne({ discordId: user.discordId, governorId });
  if (!claim) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const searchParams = req.nextUrl.searchParams;
  const yearParam = searchParams.get("year");
  const parsedYear = yearParam ? Number(yearParam) : Number.NaN;
  const targetYear = Number.isFinite(parsedYear) ? parsedYear : new Date().getUTCFullYear();

  const startInclusive = new Date(Date.UTC(targetYear, 0, 1));
  const endExclusive = Date.UTC(targetYear + 1, 0, 1);

  const projection: Document = {
    "report.metadata.parentHash": 1,
    "report.metadata.hash": 1,
    "report.metadata.email_time": 1,
    "report.metadata.start_date": 1,
    "report.self.player_id": 1,
    "report.enemy.player_id": 1,
  };

  const documents = await db
    .collection<BattleReportDocument>("battleReports")
    .find(buildMatchStage(governorId, startInclusive.getTime(), endExclusive), { projection })
    .toArray();

  const groupedByParent = new Map<
    string,
    { eventTime: number; hasNpc: boolean; hasBattle: boolean }
  >();

  for (const doc of documents) {
    const report = doc.report;
    if (!report) {
      continue;
    }

    const eventTime = extractEventTimeMillis(report);
    if (eventTime == null || eventTime < startInclusive.getTime() || eventTime >= endExclusive) {
      continue;
    }

    const selfId = Number(report.self?.player_id);
    const enemyId = Number(report.enemy?.player_id);
    if (selfId !== governorId && enemyId !== governorId) {
      continue;
    }

    const otherParticipantId = selfId === governorId ? enemyId : selfId;
    if (otherParticipantId === 0) {
      continue;
    }

    const isNpc = otherParticipantId === -2;
    const parentHash = report.metadata?.parentHash;
    const hash = report.metadata?.hash;
    const key =
      (typeof parentHash === "string" && parentHash.length > 0 && parentHash) ||
      (typeof hash === "string" && hash.length > 0 && hash) ||
      (typeof doc._id === "string" && doc._id) ||
      String(doc._id ?? eventTime);

    const existing = groupedByParent.get(key);
    if (!existing) {
      groupedByParent.set(key, {
        eventTime,
        hasNpc: isNpc,
        hasBattle: !isNpc,
      });
    } else {
      existing.eventTime = Math.min(existing.eventTime, eventTime);
      existing.hasNpc ||= isNpc;
      existing.hasBattle ||= !isNpc;
    }
  }

  const dayBuckets = new Map<string, BattleLogDay>();
  for (let cursor = new Date(startInclusive); cursor.getTime() < endExclusive; ) {
    const key = formatDayKey(cursor);
    dayBuckets.set(key, { date: key, battleCount: 0, npcCount: 0 });
    cursor = new Date(cursor.getTime() + 24 * 60 * 60 * 1000);
  }

  for (const entry of groupedByParent.values()) {
    const key = formatDayKey(new Date(entry.eventTime));
    const bucket = dayBuckets.get(key);
    if (!bucket) {
      continue;
    }

    if (entry.hasBattle) {
      bucket.battleCount += 1;
    }
    if (entry.hasNpc) {
      bucket.npcCount += 1;
    }
  }

  const days: BattleLogDay[] = Array.from(dayBuckets.values());

  return NextResponse.json(
    {
      startDate: formatDayKey(startInclusive),
      endDate: formatDayKey(new Date(endExclusive - 1)),
      days,
    },
    {
      headers: {
        "Cache-Control": "no-store",
      },
    }
  );
}
