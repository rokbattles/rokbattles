import type { Document } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import clientPromise, { toPlainObject } from "@/lib/mongo";
import type { DuelBattle2MailDocument } from "@/lib/types/duelbattle2";

export async function GET(
  _req: NextRequest,
  ctx: RouteContext<"/api/v2/olympian-arena/duel/[id]">
) {
  const { id } = await ctx.params;
  if (!id) {
    return NextResponse.json({ error: "Missing duel id" }, { status: 400 });
  }

  const parsedId = Number(id);
  if (!Number.isFinite(parsedId)) {
    return NextResponse.json({ error: "Invalid duel id" }, { status: 400 });
  }

  const duelId = parsedId;

  try {
    const mongo = await clientPromise;
    const db = mongo.db();

    const matchPipeline: Document = {
      "sender.duel.team_id": duelId,
    };

    const aggregationPipeline: Document[] = [
      { $match: matchPipeline },
      { $sort: { "metadata.mail_time": 1 } },
      {
        $project: {
          _id: { $toString: "$_id" },
          metadata: 1,
          sender: 1,
          opponent: 1,
          battle_results: 1,
        },
      },
    ];

    const documents = await db
      .collection("mails_duelbattle2")
      .aggregate(aggregationPipeline, { allowDiskUse: true })
      .toArray();

    const items: DuelBattle2MailDocument[] = documents.map(
      (doc) => toPlainObject(doc) as DuelBattle2MailDocument
    );

    return NextResponse.json(
      {
        duelId,
        items,
        count: items.length,
      },
      {
        headers: {
          "Cache-Control": "public, max-age=604800, immutable",
        },
      }
    );
  } catch (error) {
    console.error("Failed to load duel reports", error);
    return NextResponse.json(
      { error: "Failed to load duel reports" },
      {
        status: 500,
        headers: {
          "Cache-Control": "no-store",
        },
      }
    );
  }
}
