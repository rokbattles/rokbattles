import type { WithId } from "mongodb";

interface MailsBattleMetadata {
  mail_id: string;
  mail_receiver: string;
  mail_role: string | null;
  mail_time: number;
  server_id: number;
}

interface MailsBattleAlliance {
  id: number;
  name: string;
  abbreviation: string;
}

interface MailsBattleCommanderSummary {
  id: number | null;
  level: number | null;
}

interface MailsBattleParticipant {
  participant_id: number;
  player_id: number;
  player_name: string;
  alliance: {
    abbreviation: string;
  };
  commanders: {
    primary: MailsBattleCommanderSummary;
    secondary: MailsBattleCommanderSummary;
  };
}

interface MailsBattlePlayerPreview {
  player_id: number;
  player_name: string;
  kingdom_id: number | null;
  alliance: MailsBattleAlliance;
  alliance_building_id: number | null;
  tracking_key: string;
  app_id: number | null;
  app_uid: number | null;
  avatar_url: string | null;
  frame_url: string | null;
  participants: MailsBattleParticipant[];
}

interface MailsBattleOpponentPreview extends MailsBattlePlayerPreview {
  attack: {
    id: string;
    x: number;
    y: number;
  };
}

interface MailsBattleSummaryEntry {
  kill_points: number | null;
  dead: number | null;
  severely_wounded: number | null;
  slightly_wounded: number | null;
  remaining: number | null;
  troop_units: number | null;
}

interface MailsBattleSummary {
  sender: MailsBattleSummaryEntry;
  opponent: MailsBattleSummaryEntry;
}

interface MailsBattleTimelineSample {
  tick: number;
  count: number;
}

interface MailsBattleTimelineEvent {
  tick: number;
  type: number;
  event_id: number | null;
  player_id: number;
  player_name: string;
  count: number | null;
  avatar_url: string | null;
  frame_url: string | null;
  commanders: {
    primary: MailsBattleCommanderSummary;
    secondary: MailsBattleCommanderSummary;
  };
}

interface MailsBattleTimeline {
  start_timestamp: number;
  end_timestamp: number;
  start_tick: number;
  sampling: MailsBattleTimelineSample[];
  events: MailsBattleTimelineEvent[];
}

interface MailsBattleFields {
  metadata: MailsBattleMetadata;
  sender: MailsBattlePlayerPreview;
  opponents: MailsBattleOpponentPreview[];
  summary: MailsBattleSummary;
  timeline?: MailsBattleTimeline;
}

export type MailsBattleDocument = WithId<MailsBattleFields>;

export type MailsBattleListRowDb = Pick<
  MailsBattleDocument,
  "_id" | "metadata" | "sender" | "opponents" | "summary"
>;

export type MailsBattleListRow = Omit<MailsBattleListRowDb, "_id"> & {
  id: string;
};

export interface MailsBattleListDb {
  rows: MailsBattleListRowDb[];
  total: number;
}

export interface MailsBattleList {
  rows: MailsBattleListRow[];
  total: number;
}

export type MailsBattleDb = Pick<
  MailsBattleDocument,
  "_id" | "metadata" | "sender" | "opponents" | "summary" | "timeline"
>;

export type MailsBattle = Omit<MailsBattleDb, "_id"> & {
  id: string;
};
