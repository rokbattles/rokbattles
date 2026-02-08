import type { WithId } from "mongodb";

interface KingdomGovernorDataFields {
  date: Date;
  kingdom: number;
  governorId: number;
  governorName: string;
  power: number;
  allianceHelps: number;
  rssCollected: number;
  killPoints: number;
  t4Kills: number;
  t5Kills: number;
  t4Deaths: number;
  t5Deaths: number;
}

export type KingdomGovernorDataDocument = WithId<KingdomGovernorDataFields>;

export type KingdomGovernorPageRowDb = Pick<
  KingdomGovernorDataDocument,
  "_id" | "date" | "governorId" | "governorName" | "power"
>;
export type KingdomGovernorPageRow = Omit<
  KingdomGovernorPageRowDb,
  "_id" | "date"
> & {
  id: string;
  date: string;
};

export interface KingdomGovernorsPageDb {
  rows: KingdomGovernorPageRowDb[];
  total: number;
}
export interface KingdomGovernorsPage {
  rows: KingdomGovernorPageRow[];
  total: number;
}
