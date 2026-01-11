export type ReportEntry = {
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

export type BattleResultsSummary = {
  total?: BattleResultsTotals;
};

export type ReportByHashResponse = {
  parentHash: string;
  items: ReportEntry[];
  count: number;
  battleResults?: BattleResultsSummary;
};
