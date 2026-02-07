import type { WithId } from "mongodb";

interface MailsDuelBattle2Metadata {
  mail_id: string;
  mail_receiver: string;
  mail_time: number;
  server_id: number;
}

interface MailsDuelBattle2Buff {
  id: number;
  value: number;
}

interface MailsDuelBattle2CommanderSkill {
  id: number;
  level: number;
}

interface MailsDuelBattle2Commander {
  id: number;
  level: number;
  star_level: number;
  awakened: boolean;
  skills: MailsDuelBattle2CommanderSkill[];
}

interface MailsDuelBattle2Player {
  player_id: number;
  player_name: string;
  avatar_url: string | null;
  frame_url: string | null;
  alliance: {
    abbreviation: string;
  };
  duel: {
    team_id: number;
  };
  primary_commander: MailsDuelBattle2Commander;
  secondary_commander: MailsDuelBattle2Commander | null;
  buffs: MailsDuelBattle2Buff[];
}

interface MailsDuelBattle2BattleResultsEntry {
  kill_points: number;
  severely_wounded: number;
  slightly_wounded: number;
  dead: number;
  heal: number;
  units: number;
  power: number;
  win: boolean;
}

interface MailsDuelBattle2BattleResults {
  sender: MailsDuelBattle2BattleResultsEntry;
  opponent: MailsDuelBattle2BattleResultsEntry;
}

interface MailsDuelBattle2Fields {
  metadata: MailsDuelBattle2Metadata;
  battle_results: MailsDuelBattle2BattleResults;
  sender: MailsDuelBattle2Player;
  opponent: MailsDuelBattle2Player;
}

export type MailsDuelBattle2Document = WithId<MailsDuelBattle2Fields>;

export type MailsDuelBattle2PreviewDb = Pick<
  MailsDuelBattle2Document,
  "_id" | "metadata" | "sender" | "opponent"
>;

export type MailsDuelBattle2Preview = Omit<MailsDuelBattle2PreviewDb, "_id"> & {
  id: string;
};

export interface MailsDuelBattle2ListGroupRowDb {
  team_id: number;
  latest_mail_time: number;
  count: number;
  reports: MailsDuelBattle2PreviewDb[];
}

export interface MailsDuelBattle2ListGroupRow {
  team_id: number;
  latest_mail_time: number;
  count: number;
  reports: MailsDuelBattle2Preview[];
}

export interface MailsDuelBattle2ListDb {
  rows: MailsDuelBattle2ListGroupRowDb[];
  total: number;
}

export interface MailsDuelBattle2List {
  rows: MailsDuelBattle2ListGroupRow[];
  total: number;
}

export type MailsDuelBattle2Db = Pick<
  MailsDuelBattle2Document,
  "_id" | "metadata" | "sender" | "opponent"
>;

export type MailsDuelBattle2 = Omit<MailsDuelBattle2Db, "_id"> & {
  id: string;
};
