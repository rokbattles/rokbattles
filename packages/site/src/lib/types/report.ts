import type { BattleMail } from "@/lib/types/battle";

export type ReportEntry = {
  startDate: number;
  report: Record<string, unknown>;
};

export type ReportByIdResponse = {
  id: string;
  mail: BattleMail | null;
};
