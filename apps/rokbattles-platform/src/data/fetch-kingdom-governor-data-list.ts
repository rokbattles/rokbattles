import "server-only";
import { cache } from "react";
import clientPromise from "@/lib/mongo";
import type {
  KingdomGovernorDataDocument,
  KingdomGovernorsPage,
  KingdomGovernorsPageDb,
} from "@/lib/types/kingdom-governor-data";

export const fetchKingdomGovernorDataList = cache(
  async function fetchKingdomGovernorDataList(
    kingdomParam: number,
    pageParam: number,
    sizeParam: number
  ): Promise<KingdomGovernorsPage> {
    const kingdom = Number.isFinite(kingdomParam)
      ? Math.trunc(kingdomParam)
      : 0;
    const page = Math.max(1, Math.trunc(pageParam));
    const size = Math.min(100, Math.max(1, Math.trunc(sizeParam)));
    const skip = (page - 1) * size;

    if (kingdom <= 0) {
      return { rows: [], total: 0 };
    }

    const client = await clientPromise;

    if (!client) {
      return { rows: [], total: 0 };
    }

    const db = client.db();

    const result = await db
      .collection<KingdomGovernorDataDocument>("kingdomGovernorData")
      .aggregate<KingdomGovernorsPageDb>([
        {
          $match: { kingdom },
        },
        { $group: { _id: null, latestDate: { $max: "$date" } } },
        {
          $lookup: {
            from: "kingdomGovernorData",
            let: { latestDate: "$latestDate" },
            pipeline: [
              {
                $match: {
                  kingdom,
                  $expr: { $eq: ["$date", "$$latestDate"] },
                },
              },
              {
                $project: {
                  _id: 1,
                  governorId: 1,
                  governorName: 1,
                  power: 1,
                  date: 1,
                },
              },
              {
                $facet: {
                  rows: [
                    { $sort: { power: -1 } },
                    { $skip: skip },
                    { $limit: size },
                  ],
                  total: [{ $count: "value" }],
                },
              },
            ],
            as: "paged",
          },
        },
        { $unwind: "$paged" },
        {
          $project: {
            _id: 0,
            rows: "$paged.rows",
            total: { $ifNull: [{ $first: "$paged.total.value" }, 0] },
          },
        },
      ])
      .next();

    if (!result) {
      return { rows: [], total: 0 };
    }

    return {
      rows: result.rows.map((row) => ({
        id: row._id.toString(),
        governorId: row.governorId,
        governorName: row.governorName,
        power: row.power,
        date: row.date.toISOString(),
      })),
      total: result.total,
    };
  }
);
