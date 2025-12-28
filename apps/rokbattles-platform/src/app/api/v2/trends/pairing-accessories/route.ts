import type { Document } from "mongodb";
import { NextResponse } from "next/server";
import client from "@/lib/mongo";

export async function GET() {
  try {
    const mongo = await client.connect();
    const db = mongo.db();

    const doc = await db.collection<Document>("trendSnapshots").findOne(
      { trendId: "q4-2025-pairing-accessories-2" },
      {
        sort: { generatedAt: -1 },
        projection: { _id: 0 },
      }
    );

    if (!doc) {
      return NextResponse.json({ error: "Trend snapshot not found" }, { status: 404 });
    }

    const generatedAt =
      doc.generatedAt instanceof Date ? doc.generatedAt.toISOString() : doc.generatedAt;

    return NextResponse.json(
      {
        ...doc,
        generatedAt,
      },
      {
        headers: {
          "Cache-Control": "no-store",
        },
      }
    );
  } catch (error) {
    console.error("Failed to load trend snapshot", error);
    return NextResponse.json({ error: "Failed to load trend snapshot" }, { status: 500 });
  }
}
