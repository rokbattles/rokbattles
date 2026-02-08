import "server-only";
import { cache } from "react";
import clientPromise from "@/lib/mongo";
import type {
  MailsDuelBattle2,
  MailsDuelBattle2Db,
  MailsDuelBattle2Document,
} from "@/lib/types/mails-duelbattle2";

export const fetchMailsDuelBattle2 = cache(async function fetchMailsDuelBattle2(
  mailIdParam: string
): Promise<MailsDuelBattle2 | null> {
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
    .collection<MailsDuelBattle2Document>("mails_duelbattle2")
    .findOne<MailsDuelBattle2Db>(
      { "metadata.mail_id": mailId },
      {
        projection: {
          _id: 1,
          metadata: 1,
          sender: 1,
          opponent: 1,
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
    opponent: report.opponent,
  };
});
