import "server-only";

import type { Db, Document } from "mongodb";

type LootEntryDocument = {
  type?: unknown;
  sub_type?: unknown;
  value?: unknown;
};

type BattleOpponentDocument = {
  player_id?: unknown;
  npc?: {
    type?: unknown;
    b_type?: unknown;
    loot?: LootEntryDocument[] | null;
  } | null;
};

export type BattleMailDocument = {
  metadata?: {
    mail_time?: unknown;
  };
  opponents?: BattleOpponentDocument[] | null;
};

export type BarbarianFortMailDocument = {
  metadata?: {
    mail_time?: unknown;
  };
  rewards?: LootEntryDocument[] | null;
};

type BaulurParticipantDocument = {
  player_id?: unknown;
  loot?: LootEntryDocument[] | null;
};

export type BaulurMailDocument = {
  metadata?: {
    mail_time?: unknown;
  };
  participants?: BaulurParticipantDocument[] | null;
};

const battleProjection: Document = {
  _id: 0,
  "metadata.mail_time": 1,
  "opponents.player_id": 1,
  "opponents.npc.type": 1,
  "opponents.npc.b_type": 1,
  "opponents.npc.loot": 1,
};

const barbarianFortProjection: Document = {
  _id: 0,
  "metadata.mail_time": 1,
  rewards: 1,
};

const baulurProjection: Document = {
  _id: 0,
  "metadata.mail_time": 1,
  "participants.player_id": 1,
  "participants.loot": 1,
};

export async function fetchBarbarianBattleMails(
  db: Db,
  options: { mailReceiver: string; timeMatch: Document }
): Promise<BattleMailDocument[]> {
  const { mailReceiver, timeMatch } = options;

  return db
    .collection<BattleMailDocument>("mails_battle")
    .find(
      {
        $and: [
          { "metadata.mail_receiver": mailReceiver },
          { opponents: { $elemMatch: { player_id: -2, "npc.b_type": 1 } } },
          timeMatch,
        ],
      },
      { projection: battleProjection }
    )
    .toArray();
}

export async function fetchBarbarianFortMails(
  db: Db,
  options: { mailReceiver: string; timeMatch: Document }
): Promise<BarbarianFortMailDocument[]> {
  const { mailReceiver, timeMatch } = options;

  return db
    .collection<BarbarianFortMailDocument>("mails_system_barbarianfort")
    .find(
      {
        $and: [{ "metadata.mail_receiver": mailReceiver }, timeMatch],
      },
      { projection: barbarianFortProjection }
    )
    .toArray();
}

export async function fetchBaulurMails(
  db: Db,
  options: { mailReceiver: string; timeMatch: Document; governorId: number }
): Promise<BaulurMailDocument[]> {
  const { mailReceiver, timeMatch, governorId } = options;

  return db
    .collection<BaulurMailDocument>("mails_barcanyonkillboss")
    .find(
      {
        $and: [
          { "metadata.mail_receiver": mailReceiver },
          { participants: { $elemMatch: { player_id: governorId } } },
          timeMatch,
        ],
      },
      { projection: baulurProjection }
    )
    .toArray();
}
