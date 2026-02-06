import type { ObjectId } from "mongodb";

export interface ExploreCommanderPair {
  primary: number | null;
  secondary: number | null;
}

export interface ExploreBattleReportRowDb {
  _id: ObjectId;
  mail_id: string | null;
  start_timestamp: number | null;
  end_timestamp: number | null;
  sender_commanders: ExploreCommanderPair;
  opponent_commanders: ExploreCommanderPair;
  trade_percentage: number;
  battles: number;
}

export interface ExploreBattleReportsPageDb {
  rows: ExploreBattleReportRowDb[];
  total: number;
}

export interface ExploreBattleReportRow {
  id: string;
  mailId: string;
  startTimestamp: number | null;
  endTimestamp: number | null;
  senderCommanders: ExploreCommanderPair;
  opponentCommanders: ExploreCommanderPair;
  tradePercentage: number;
  battles: number;
}

export interface ExploreBattleReportsPage {
  rows: ExploreBattleReportRow[];
  total: number;
}
