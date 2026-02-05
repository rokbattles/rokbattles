import "server-only";
import { cache } from "react";
import clientPromise from "@/lib/mongo";
import type {
  MailsBattle,
  MailsBattleDb,
  MailsBattleDocument,
} from "@/lib/types/mails-battle";

export const fetchMailsBattle = cache(async function fetchMailsBattle(
  mailIdParam: string
): Promise<MailsBattle | null> {
  const mailId = mailIdParam.trim();

  if (!mailId) {
    return null;
  }

  const client = await clientPromise;

  if (!client) {
    return null;
  }

  const db = client.db();

  const report = await db
    .collection<MailsBattleDocument>("mails_battle")
    .findOne<MailsBattleDb>(
      { "metadata.mail_id": mailId },
      {
        projection: {
          _id: 1,
          metadata: 1,
          sender: 1,
          opponents: 1,
          summary: 1,
          timeline: 1,
        },
      }
    );

  if (!report) {
    return null;
  }

  return {
    id: report._id.toString(),
    metadata: report.metadata,
    sender: report.sender,
    opponents: report.opponents,
    summary: report.summary,
    timeline: report.timeline,
  };
});
