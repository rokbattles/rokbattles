export interface ReportEntry {
  startDate: number;
  report: Record<string, unknown>;
}

export interface ReportMergeItem {
  parentHash: string;
  latestEmailTime: number;
}

export interface ReportMergeSummary {
  trackingKey: string;
  reports: ReportMergeItem[];
}

export interface BattleResultsTotals {
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
}

export interface BattleResultsSummary {
  total?: BattleResultsTotals;
}

export interface ReportByHashResponse {
  parentHash: string;
  items: ReportEntry[];
  count: number;
  battleResults?: BattleResultsSummary;
  merge?: ReportMergeSummary;
}
