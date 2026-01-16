export interface AccessoryPairCount {
  ids: [number, number];
  count: number;
}

export interface AccessoryCount {
  id: number;
  count: number;
}

export interface PairingSnapshot {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  reportCount: number;
  accessorySampleCount: number;
  accessoryPairs: AccessoryPairCount[];
  accessories: AccessoryCount[];
}

export interface CategorySnapshot {
  totalReports: number;
  pairings: PairingSnapshot[];
}

export interface TrendSnapshot {
  trendId: string;
  generatedAt?: string;
  period?: {
    label?: string;
    startMs?: number;
    endMs?: number;
  };
  minReportCount?: number;
  minReportCounts?: {
    field?: number;
    rally?: number;
    garrison?: number;
  };
  categories?: {
    field?: CategorySnapshot;
    rally?: CategorySnapshot;
    garrison?: CategorySnapshot;
  };
}

export type CategoryKey = "field" | "rally" | "garrison";
