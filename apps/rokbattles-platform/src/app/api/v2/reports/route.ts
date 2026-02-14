import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import clientPromise from "@/lib/mongo";

type ReportsFilterType = "home" | "ark" | "kvk" | "strife";
type ReportsFilterSide = "none" | "sender" | "opponent" | "both";
type ReportsGarrisonBuildingType = "flag" | "fortress" | "other";

const INVALID_OPPONENT_PLAYER_IDS = new Set([-2, 0]);

type CommanderLike = {
  id?: unknown;
};

type OpponentBattleResultLike = {
  sender?: {
    dead?: unknown;
    severely_wounded?: unknown;
    kill_points?: unknown;
  } | null;
  opponent?: {
    dead?: unknown;
    severely_wounded?: unknown;
    kill_points?: unknown;
  } | null;
};

type OpponentLike = {
  player_id?: unknown;
  start_tick?: unknown;
  rally?: unknown;
  alliance_building_id?: unknown;
  commanders?: {
    primary?: CommanderLike | null;
    secondary?: CommanderLike | null;
  } | null;
  battle_results?: OpponentBattleResultLike | null;
};

type BattleSummaryLike = {
  sender?: {
    dead?: unknown;
    severely_wounded?: unknown;
    kill_points?: unknown;
  } | null;
  opponent?: {
    dead?: unknown;
    severely_wounded?: unknown;
    kill_points?: unknown;
  } | null;
};

function toFiniteNumber(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function computeTradePercent(senderKillPoints: number, opponentKillPoints: number): number {
  if (senderKillPoints === opponentKillPoints) {
    return 100;
  }

  if (opponentKillPoints <= 0) {
    return 0;
  }

  return Math.round((senderKillPoints / opponentKillPoints) * 100);
}

function parseNumberParam(
  value: string | null,
  error: string
): { value?: number; response?: NextResponse } {
  if (!value) {
    return {};
  }

  const parsedValue = Number(value);
  if (Number.isFinite(parsedValue)) {
    return { value: parsedValue };
  }

  return {
    response: NextResponse.json({ error }, { status: 400 }),
  };
}

function parseFilterType(value: string | null): {
  value?: ReportsFilterType;
  response?: NextResponse;
} {
  if (!value) {
    return {};
  }

  if (value === "home" || value === "ark" || value === "kvk" || value === "strife") {
    return { value };
  }

  return {
    response: NextResponse.json({ error: "Invalid type" }, { status: 400 }),
  };
}

function parseFilterSide(
  value: string | null,
  error: string
): { value?: ReportsFilterSide; response?: NextResponse } {
  if (!value) {
    return {};
  }

  if (value === "none" || value === "sender" || value === "opponent" || value === "both") {
    return { value };
  }

  return {
    response: NextResponse.json({ error }, { status: 400 }),
  };
}

function parseGarrisonBuildingType(value: string | null): {
  value?: ReportsGarrisonBuildingType;
  response?: NextResponse;
} {
  if (!value) {
    return {};
  }

  if (value === "flag" || value === "fortress" || value === "other") {
    return { value };
  }

  return {
    response: NextResponse.json({ error: "Invalid garrison building" }, { status: 400 }),
  };
}

function isStrifeReportCondition() {
  return {
    $and: [
      { "sender.supreme_strife.battle_id": { $exists: true, $nin: [null, ""] } },
      { "sender.supreme_strife.team_id": { $exists: true, $nin: [null, 0] } },
    ],
  };
}

function buildGarrisonFieldCondition(
  path: string,
  type: ReportsGarrisonBuildingType | undefined
): Document {
  if (type === "flag") {
    return { [path]: 1 };
  }
  if (type === "fortress") {
    return { [path]: 3 };
  }
  if (type === "other") {
    return { [path]: { $exists: true, $nin: [1, 3, null] } };
  }
  return { [path]: { $exists: true, $ne: null } };
}

function buildOpponentGarrisonCondition(type: ReportsGarrisonBuildingType | undefined): Document {
  if (type === "flag") {
    return {
      opponents: {
        $elemMatch: { player_id: { $nin: [-2, 0] }, alliance_building_id: 1 },
      },
    };
  }
  if (type === "fortress") {
    return {
      opponents: {
        $elemMatch: { player_id: { $nin: [-2, 0] }, alliance_building_id: 3 },
      },
    };
  }
  if (type === "other") {
    return {
      opponents: {
        $elemMatch: {
          player_id: { $nin: [-2, 0] },
          alliance_building_id: { $exists: true, $nin: [1, 3, null] },
        },
      },
    };
  }
  return {
    opponents: {
      $elemMatch: {
        player_id: { $nin: [-2, 0] },
        alliance_building_id: { $exists: true, $ne: null },
      },
    },
  };
}

function getValidOpponents(value: unknown): OpponentLike[] {
  if (!Array.isArray(value)) {
    return [];
  }

  return value.filter((opponent): opponent is OpponentLike => {
    const playerId = toFiniteNumber((opponent as OpponentLike).player_id);
    return !INVALID_OPPONENT_PLAYER_IDS.has(playerId);
  });
}

function getSortedOpponents(opponents: OpponentLike[]): OpponentLike[] {
  return [...opponents].sort((a, b) => {
    const tickDelta = toFiniteNumber(a.start_tick) - toFiniteNumber(b.start_tick);
    if (tickDelta !== 0) {
      return tickDelta;
    }
    return toFiniteNumber(a.player_id) - toFiniteNumber(b.player_id);
  });
}

function computeKillAndTradePercent(summary: unknown, opponents: OpponentLike[]) {
  const summaryData = (summary ?? null) as BattleSummaryLike | null;
  const senderSummary = summaryData?.sender ?? null;
  const opponentSummary = summaryData?.opponent ?? null;
  const opponentSeverelyWounded = opponentSummary?.severely_wounded;
  const opponentDead = opponentSummary?.dead;
  const senderKillPoints = senderSummary?.kill_points;
  const opponentKillPoints = opponentSummary?.kill_points;
  const hasSummaryMetrics =
    typeof senderSummary === "object" &&
    typeof opponentSummary === "object" &&
    isFiniteNumber(opponentSeverelyWounded) &&
    isFiniteNumber(opponentDead) &&
    isFiniteNumber(senderKillPoints) &&
    isFiniteNumber(opponentKillPoints);

  if (hasSummaryMetrics) {
    const opponentKillCount = opponentSeverelyWounded + opponentDead;

    return {
      killCount: opponentKillCount,
      tradePercent: computeTradePercent(senderKillPoints, opponentKillPoints),
    };
  }

  const aggregated = opponents.reduce(
    (totals, opponent) => {
      const senderResult = opponent.battle_results?.sender;
      const opponentResult = opponent.battle_results?.opponent;

      totals.killCount +=
        toFiniteNumber(opponentResult?.severely_wounded) + toFiniteNumber(opponentResult?.dead);
      totals.senderKillPoints += toFiniteNumber(senderResult?.kill_points);
      totals.opponentKillPoints += toFiniteNumber(opponentResult?.kill_points);
      return totals;
    },
    { killCount: 0, senderKillPoints: 0, opponentKillPoints: 0 }
  );

  return {
    killCount: aggregated.killCount,
    tradePercent: computeTradePercent(aggregated.senderKillPoints, aggregated.opponentKillPoints),
  };
}

export async function GET(req: NextRequest) {
  const searchParams = req.nextUrl.searchParams;

  const cursor = searchParams.get("cursor");
  const type = searchParams.get("type");
  const playerId = searchParams.get("pid");
  const senderPrimaryCommanderId = searchParams.get("spc");
  const senderSecondaryCommanderId = searchParams.get("ssc");
  const opponentPrimaryCommanderId = searchParams.get("opc");
  const opponentSecondaryCommanderId = searchParams.get("osc");
  const rallySideParam = searchParams.get("rs");
  const garrisonSideParam = searchParams.get("gs");
  const garrisonBuildingParam = searchParams.get("gb");

  const parsedTypeResult = parseFilterType(type);
  if (parsedTypeResult.response) return parsedTypeResult.response;
  const parsedType = parsedTypeResult.value;

  const parsedCursorResult = parseNumberParam(cursor, "Invalid cursor");
  if (parsedCursorResult.response) return parsedCursorResult.response;
  const parsedCursor = parsedCursorResult.value;

  const parsedPlayerIdResult = parseNumberParam(playerId, "Invalid governor id");
  if (parsedPlayerIdResult.response) return parsedPlayerIdResult.response;
  const parsedPlayerId = parsedPlayerIdResult.value;

  const parsedSenderPrimaryCommanderIdResult = parseNumberParam(
    senderPrimaryCommanderId,
    "Invalid sender primary commander id"
  );
  if (parsedSenderPrimaryCommanderIdResult.response)
    return parsedSenderPrimaryCommanderIdResult.response;
  const parsedSenderPrimaryCommanderId = parsedSenderPrimaryCommanderIdResult.value;

  const parsedSenderSecondaryCommanderIdResult = parseNumberParam(
    senderSecondaryCommanderId,
    "Invalid sender secondary commander id"
  );
  if (parsedSenderSecondaryCommanderIdResult.response)
    return parsedSenderSecondaryCommanderIdResult.response;
  const parsedSenderSecondaryCommanderId = parsedSenderSecondaryCommanderIdResult.value;

  const parsedOpponentPrimaryCommanderIdResult = parseNumberParam(
    opponentPrimaryCommanderId,
    "Invalid opponent primary commander id"
  );
  if (parsedOpponentPrimaryCommanderIdResult.response)
    return parsedOpponentPrimaryCommanderIdResult.response;
  const parsedOpponentPrimaryCommanderId = parsedOpponentPrimaryCommanderIdResult.value;

  const parsedOpponentSecondaryCommanderIdResult = parseNumberParam(
    opponentSecondaryCommanderId,
    "Invalid opponent secondary commander id"
  );
  if (parsedOpponentSecondaryCommanderIdResult.response)
    return parsedOpponentSecondaryCommanderIdResult.response;
  const parsedOpponentSecondaryCommanderId = parsedOpponentSecondaryCommanderIdResult.value;

  const parsedRallySideResult = parseFilterSide(rallySideParam, "Invalid rally side");
  if (parsedRallySideResult.response) return parsedRallySideResult.response;
  const parsedRallySide = parsedRallySideResult.value ?? "none";

  const parsedGarrisonSideResult = parseFilterSide(garrisonSideParam, "Invalid garrison side");
  if (parsedGarrisonSideResult.response) return parsedGarrisonSideResult.response;
  const parsedGarrisonSide = parsedGarrisonSideResult.value ?? "none";

  const parsedGarrisonBuildingResult = parseGarrisonBuildingType(garrisonBuildingParam);
  if (parsedGarrisonBuildingResult.response) return parsedGarrisonBuildingResult.response;
  const parsedGarrisonBuilding = parsedGarrisonBuildingResult.value;

  const sideOverlaps =
    ((parsedRallySide === "sender" || parsedRallySide === "both") &&
      (parsedGarrisonSide === "sender" || parsedGarrisonSide === "both")) ||
    ((parsedRallySide === "opponent" || parsedRallySide === "both") &&
      (parsedGarrisonSide === "opponent" || parsedGarrisonSide === "both"));
  if (sideOverlaps) {
    return NextResponse.json(
      { error: "Rally and garrison cannot overlap on the same side" },
      { status: 400 }
    );
  }

  const mongo = await clientPromise;
  const db = mongo.db();

  const matchPipeline: Document[] = [
    { opponents: { $elemMatch: { player_id: { $nin: [-2, 0] } } } },
  ];

  if (parsedCursor !== undefined) {
    matchPipeline.push({ "metadata.mail_time": { $lt: parsedCursor } });
  }

  if (parsedPlayerId !== undefined) {
    matchPipeline.push({
      $or: [{ "sender.player_id": parsedPlayerId }, { "opponents.player_id": parsedPlayerId }],
    });
  }

  if (parsedSenderPrimaryCommanderId !== undefined) {
    matchPipeline.push({
      "sender.commanders.primary.id": parsedSenderPrimaryCommanderId,
    });
  }

  if (parsedSenderSecondaryCommanderId !== undefined) {
    matchPipeline.push({
      "sender.commanders.secondary.id": parsedSenderSecondaryCommanderId,
    });
  }

  if (parsedOpponentPrimaryCommanderId !== undefined) {
    matchPipeline.push({
      opponents: {
        $elemMatch: {
          player_id: { $nin: [-2, 0] },
          "commanders.primary.id": parsedOpponentPrimaryCommanderId,
        },
      },
    });
  }

  if (parsedOpponentSecondaryCommanderId !== undefined) {
    matchPipeline.push({
      opponents: {
        $elemMatch: {
          player_id: { $nin: [-2, 0] },
          "commanders.secondary.id": parsedOpponentSecondaryCommanderId,
        },
      },
    });
  }

  if (parsedType) {
    if (parsedType === "kvk") {
      matchPipeline.push({ "metadata.kvk": true });
    }

    if (parsedType === "ark") {
      matchPipeline.push({ "metadata.mail_role": "dungeon" });
    }

    if (parsedType === "home") {
      matchPipeline.push({
        $and: [
          { "metadata.kvk": { $ne: true } },
          { "metadata.mail_role": { $ne: "dungeon" } },
          {
            $or: [
              { "sender.supreme_strife.battle_id": { $in: [null, ""] } },
              { "sender.supreme_strife.team_id": { $in: [null, 0] } },
            ],
          },
        ],
      });
    }

    if (parsedType === "strife") {
      matchPipeline.push(isStrifeReportCondition());
    }
  }

  const rallyConditions: Document[] = [];
  if (parsedRallySide === "sender" || parsedRallySide === "both") {
    rallyConditions.push({ "sender.rally": { $in: [1, true] } });
  }
  if (parsedRallySide === "opponent" || parsedRallySide === "both") {
    rallyConditions.push({
      opponents: {
        $elemMatch: { player_id: { $nin: [-2, 0] }, rally: { $in: [1, true] } },
      },
    });
  }
  if (rallyConditions.length === 1) {
    matchPipeline.push(rallyConditions[0]);
  } else if (rallyConditions.length > 1) {
    matchPipeline.push({ $or: rallyConditions });
  }

  const garrisonConditions: Document[] = [];
  if (parsedGarrisonSide === "sender" || parsedGarrisonSide === "both") {
    garrisonConditions.push(
      buildGarrisonFieldCondition("sender.alliance_building_id", parsedGarrisonBuilding)
    );
  }
  if (parsedGarrisonSide === "opponent" || parsedGarrisonSide === "both") {
    garrisonConditions.push(buildOpponentGarrisonCondition(parsedGarrisonBuilding));
  }
  if (garrisonConditions.length === 1) {
    matchPipeline.push(garrisonConditions[0]);
  } else if (garrisonConditions.length > 1) {
    matchPipeline.push({ $or: garrisonConditions });
  }

  const finalMatchPipeline =
    matchPipeline.length === 1 ? matchPipeline[0] : { $and: matchPipeline };

  const aggregationPipeline: Document[] = [
    { $match: finalMatchPipeline },
    { $sort: { "metadata.mail_time": -1 } },
    { $limit: 101 },
    {
      $project: {
        _id: 0,
        mailId: "$metadata.mail_id",
        mailTime: "$metadata.mail_time",
        timelineStart: "$timeline.start_timestamp",
        timelineEnd: "$timeline.end_timestamp",
        senderPrimaryCommanderId: "$sender.commanders.primary.id",
        senderSecondaryCommanderId: "$sender.commanders.secondary.id",
        opponents: "$opponents",
        summary: "$summary",
      },
    },
  ];

  const documents = await db
    .collection("mails_battle")
    .aggregate(aggregationPipeline, { allowDiskUse: true })
    .toArray();

  const hasMore = documents.length > 100;
  const finalDocuments = hasMore ? documents.slice(0, 100) : documents;
  const finalCursor: string | undefined = hasMore
    ? String(toFiniteNumber(finalDocuments[finalDocuments.length - 1].mailTime))
    : undefined;

  const items = finalDocuments
    .map((document) => {
      const mailId = typeof document.mailId === "string" ? document.mailId : "";
      if (mailId === "") {
        return null;
      }

      const validOpponents = getSortedOpponents(getValidOpponents(document.opponents));
      const firstOpponent = validOpponents[0];

      if (!firstOpponent) {
        return null;
      }

      const { killCount, tradePercent } = computeKillAndTradePercent(
        document.summary,
        validOpponents
      );

      return {
        mailId,
        timeStart: toFiniteNumber(document.timelineStart),
        timeEnd: toFiniteNumber(document.timelineEnd),
        sender: {
          primaryCommanderId: toFiniteNumber(document.senderPrimaryCommanderId),
          secondaryCommanderId: toFiniteNumber(document.senderSecondaryCommanderId),
        },
        opponent: {
          primaryCommanderId: toFiniteNumber(firstOpponent.commanders?.primary?.id),
          secondaryCommanderId: toFiniteNumber(firstOpponent.commanders?.secondary?.id),
        },
        battles: validOpponents.length,
        killCount,
        tradePercent,
      };
    })
    .filter((item): item is NonNullable<typeof item> => item !== null);

  return NextResponse.json(
    {
      items,
      count: items.length,
      cursor: finalCursor,
    },
    {
      headers: {
        "Cache-Control": "no-store",
      },
    }
  );
}
