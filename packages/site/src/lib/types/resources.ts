export type ResourceTotals = {
  gain: number;
  bonus: number;
  total: number;
};

export type ResourceTotalsByType = ResourceTotals & {
  type: number;
};

export type ResourceDailyValueByType = {
  type: number;
  total: number;
};

export type ResourcesDailyAggregate = {
  date: string;
  crystalsGain: number;
  resources: ResourceDailyValueByType[];
};

export type ResourcesQueryResult = {
  range: {
    start: string;
    end: string;
  };
  totalReports: number;
  crystalsGain: ResourceTotals;
  resources: ResourceTotalsByType[];
  daily: ResourcesDailyAggregate[];
};
