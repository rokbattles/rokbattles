export type AccessoryPairCount = {
  ids: [number, number];
  count: number;
};

export type AccessoryCount = {
  id: number;
  count: number;
};

export type PairingSnapshot = {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  reportCount: number;
  accessorySampleCount: number;
  accessoryPairs: AccessoryPairCount[];
  accessories: AccessoryCount[];
};

export type CategorySnapshot = {
  totalReports: number;
  pairings: PairingSnapshot[];
};

export type TrendSnapshot = {
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
};

export type CategoryKey = "field" | "rally" | "garrison";
