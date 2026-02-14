import { type NextRequest, NextResponse } from "next/server";
import clientPromise, { toPlainObject } from "@/lib/mongo";
import type { BattleMail } from "@/lib/types/battle";
import type { ReportByIdResponse } from "@/lib/types/report";

export async function GET(_req: NextRequest, ctx: RouteContext<"/api/v2/report/[id]">) {
  const { id } = await ctx.params;
  if (!id) {
    return NextResponse.json({ error: "Missing report id" }, { status: 400 });
  }

  try {
    const mongo = await clientPromise;
    const db = mongo.db();

    const document = await db.collection("mails_battle").findOne(
      {
        "metadata.mail_id": id,
      },
      {
        projection: {
          _id: 0,
          metadata: 1,
          sender: 1,
          summary: 1,
          opponents: 1,
          timeline: 1,
        },
      }
    );

    const mail = document ? (toPlainObject(document) as BattleMail) : null;
    const payload: ReportByIdResponse = {
      id,
      mail,
    };

    return NextResponse.json(payload, {
      headers: {
        "Cache-Control": "public, max-age=604800, immutable",
      },
    });
  } catch (error) {
    console.error("Failed to load report by id", error);
    return NextResponse.json(
      { error: "Failed to load report" },
      {
        status: 500,
        headers: {
          "Cache-Control": "no-store",
        },
      }
    );
  }
}
