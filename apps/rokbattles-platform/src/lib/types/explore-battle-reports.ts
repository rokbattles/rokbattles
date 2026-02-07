import type { ObjectId } from "mongodb";

export interface ExploreCommanderPair {
  primary: number | null;
  secondary: number | null;
}

export interface ExploreBattleSummaryEntryDb {
  dead: number | null;
  kill_points: number | null;
  remaining: number | null;
  severely_wounded: number | null;
  slightly_wounded: number | null;
  troop_units: number | null;
}

export interface ExploreBattleSummaryDb {
  sender: ExploreBattleSummaryEntryDb;
  opponent: ExploreBattleSummaryEntryDb;
}

export interface ExploreBattleTimelineSampleDb {
  tick: number;
  count: number;
}

export interface ExploreBattleTimelineDb {
  start_timestamp: number | null;
  end_timestamp: number | null;
  sampling: ExploreBattleTimelineSampleDb[];
}

export interface ExploreBattleReportRowDb {
  _id: ObjectId;
  mail_id: string | null;
  mail_time: number;
  start_timestamp: number | null;
  end_timestamp: number | null;
  sender_commanders: ExploreCommanderPair;
  opponent_commanders: ExploreCommanderPair;
  trade_percentage: number;
  battles: number;
  summary: ExploreBattleSummaryDb;
  timeline: ExploreBattleTimelineDb;
}

export interface ExploreBattleReportsPageDb {
  rows: ExploreBattleReportRowDb[];
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
  summary: ExploreBattleSummary;
  timeline: ExploreBattleTimeline;
}

export interface ExploreBattleSummaryEntry {
  dead: number | null;
  killPoints: number | null;
  remaining: number | null;
  severelyWounded: number | null;
  slightlyWounded: number | null;
  troopUnits: number | null;
}

export interface ExploreBattleSummary {
  sender: ExploreBattleSummaryEntry;
  opponent: ExploreBattleSummaryEntry;
}

export interface ExploreBattleTimelineSample {
  tick: number;
  count: number;
}

export interface ExploreBattleTimeline {
  startTimestamp: number | null;
  endTimestamp: number | null;
  sampling: ExploreBattleTimelineSample[];
}

export interface ExploreBattleReportsPage {
  rows: ExploreBattleReportRow[];
  nextAfter: string | null;
  previousBefore: string | null;
}
