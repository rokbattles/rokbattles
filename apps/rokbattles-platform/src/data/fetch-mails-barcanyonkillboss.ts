import "server-only";
import { cache } from "react";
import clientPromise from "@/lib/mongo";
import type {
  MailsBarCanyonKillBoss,
  MailsBarCanyonKillBossDb,
  MailsBarCanyonKillBossDocument,
} from "@/lib/types/mails-barcanyonkillboss";

export const fetchMailsBarCanyonKillBoss = cache(
  async function fetchMailsBarCanyonKillBoss(
    mailIdParam: string
  ): Promise<MailsBarCanyonKillBoss | null> {
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
      .collection<MailsBarCanyonKillBossDocument>("mails_barcanyonkillboss")
      .findOne<MailsBarCanyonKillBossDb>(
        { "metadata.mail_id": mailId },
        {
          projection: {
            _id: 1,
            metadata: 1,
            npc: 1,
            participants: 1,
          },
        }
      );

    if (!report) {
      return null;
    }

    return {
      id: report._id.toString(),
      metadata: report.metadata,
      npc: report.npc,
      participants: report.participants,
    };
  }
);
