export type ReportEntry = {
  hash: string;
  startDate: number;
  report: Record<string, unknown>;
};

export type BattleResultsTotals = {
  death: number;
  severelyWounded: number;
  wounded: number;
  remaining: number;
  killScore: number;
  enemyDeath: number;
  enemySeverelyWounded: number;
  enemyWounded: number;
  enemyRemaining: number;
  enemyKillScore: number;
};

export type BattleResultsTimelineEntry = BattleResultsTotals & {
  startDate: number;
  endDate: number | null;
};

export type BattleResultsSummary = {
  total?: BattleResultsTotals;
  timeline: BattleResultsTimelineEntry[];
};

export type ReportByHashResponse = {
  parentHash: string;
  items: ReportEntry[];
  count: number;
  battleResults?: BattleResultsSummary;
};
