export type CarriageResource = {
  type: number;
  amount: number;
};

export type CarriageCounterparty = {
  governorId: number;
  name: string;
  resources: CarriageResource[];
};

export type CarriageActivity = CarriageCounterparty & {
  id: string;
  direction: "received" | "sent";
  newestAt: number;
  oldestAt: number;
  reportCount: number;
  tax?: { min: number; max: number };
};

export type CarriageResponse = {
  totalReceived: CarriageResource[];
  totalSent: CarriageResource[];
  topReceived: CarriageCounterparty[];
  topSent: CarriageCounterparty[];
  activity: CarriageActivity[];
  page: number;
  totalPages: number;
  totalGroups: number;
};
