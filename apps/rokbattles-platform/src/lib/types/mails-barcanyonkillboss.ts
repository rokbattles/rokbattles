import type { WithId } from "mongodb";

interface MailsBarCanyonKillBossMetadata {
  mail_id: string;
  mail_receiver: string;
  mail_time: number;
  server_id: number;
}

interface MailsBarCanyonKillBossNpcLocation {
  x: number;
  y: number;
}

interface MailsBarCanyonKillBossNpc {
  type: number;
  level: number;
  location: MailsBarCanyonKillBossNpcLocation;
}

interface MailsBarCanyonKillBossLoot {
  type: number;
  sub_type: number;
  value: number;
}

interface MailsBarCanyonKillBossParticipant {
  player_id: number;
  player_name: string;
  avatar_url: string | null;
  frame_url: string | null;
  damage_rate: number;
  loot: MailsBarCanyonKillBossLoot[];
}

interface MailsBarCanyonKillBossFields {
  metadata: MailsBarCanyonKillBossMetadata;
  npc: MailsBarCanyonKillBossNpc;
  participants: MailsBarCanyonKillBossParticipant[];
}

export type MailsBarCanyonKillBossDocument =
  WithId<MailsBarCanyonKillBossFields>;

export type MailsBarCanyonKillBossListRowDb = Pick<
  MailsBarCanyonKillBossDocument,
  "_id" | "metadata" | "npc" | "participants"
>;

export type MailsBarCanyonKillBossListRow = Omit<
  MailsBarCanyonKillBossListRowDb,
  "_id"
> & {
  id: string;
};

export interface MailsBarCanyonKillBossListDb {
  rows: MailsBarCanyonKillBossListRowDb[];
  total: number;
}

export interface MailsBarCanyonKillBossList {
  rows: MailsBarCanyonKillBossListRow[];
  total: number;
}

export type MailsBarCanyonKillBossDb = Pick<
  MailsBarCanyonKillBossDocument,
  "_id" | "metadata" | "npc" | "participants"
>;

export type MailsBarCanyonKillBoss = Omit<MailsBarCanyonKillBossDb, "_id"> & {
  id: string;
};
