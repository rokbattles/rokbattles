export type ReportsListParticipant = {
  primaryCommanderId: number;
  primaryCommanderAwakened: boolean | null;
  secondaryCommanderId: number;
  secondaryCommanderAwakened: boolean | null;
};

export type ReportsSummaryEntry = {
  troopUnits: number;
  dead: number;
  severelyWounded: number;
  slightlyWounded: number;
  remaining: number;
  killPoints: number;
};

export type ReportsTimelineSample = {
  tick: number;
  count: number;
};

export type ReportsTimeline = {
  startTimestamp: number;
  endTimestamp: number;
  sampling: ReportsTimelineSample[];
};

export type ReportsListItem = {
  mailId: string;
  timeStart: number;
  timeEnd: number;
  sender: ReportsListParticipant;
  opponent: ReportsListParticipant;
  battles: number;
  killCount: number;
  tradePercent: number;
  summary: {
    sender: ReportsSummaryEntry;
    opponent: ReportsSummaryEntry;
  };
  timeline: ReportsTimeline;
};

export type ReportsListResponse = {
  items: ReportsListItem[];
  nextAfter: string | null;
  previousBefore: string | null;
};
