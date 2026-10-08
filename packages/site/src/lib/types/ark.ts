import type { ReportsListItem } from "@/lib/types/reports-list";

export type ArkMatchAlliance = {
  id: number | null;
  name: string | null;
  logo: string | null;
  abbreviation: string | null;
  score: number | null;
  members: number | null;
  membersMax: number | null;
  isBlue: boolean | null;
};

export type ArkMatchRecord = {
  matchId: string;
  mailTimeMillis: number;
  alliances: ArkMatchAlliance[];
  winnerAllianceId: number | null;
  selfAllianceId: number | null;
  league: ArkLeague;
  personalScore: number | null;
  hasIndividualResults: boolean;
};

export type ArkMatchHistoryResult = {
  limit: number;
  total: number;
  items: ArkMatchRecord[];
};

export type ArkMatchDetailOverview = {
  rank: number | null;
  score: number | null;
};

export type ArkMatchDetailIndividualResults = {
  battlesWin: number | null;
  battlesLose: number | null;
  winRate: number | null;
  kills: number | null;
  severelyWounded: number | null;
  unitsHealed: number | null;
  speedupsMinutes: number | null;
  teleports: number | null;
  provisionsScore: number | null;
  arkOfOsirisScore: number | null;
  killScore: number | null;
  occupationScore: number | null;
  healingScore: number | null;
};

export type ArkMatchDetailPairing = {
  primaryCommanderId: number | null;
  secondaryCommanderId: number | null;
  battles: number | null;
  battlesWin: number | null;
  killCount: number | null;
  killPoints: number | null;
  lossPoints: number | null;
};

export type ArkMatchDetail = ArkMatchRecord & {
  overview: ArkMatchDetailOverview;
  individualResults: ArkMatchDetailIndividualResults;
  pairings: ArkMatchDetailPairing[];
  participants: ArkParticipant[];
  highlights: ArkHighlight[];
  battleReports: ArkReportCoverage;
};

export type ArkMatchDetailResponse = {
  id: string;
  match: ArkMatchDetail | null;
};

export type ArkLeague = "golden" | "silver" | "osiris" | "practice" | "custom" | "unknown";

export type ArkParticipant = {
  participated: boolean | null;
  rank: number;
  name: string | null;
  score: number | null;
  occupationScore: number | null;
  provisionsScore: number | null;
  killScore: number | null;
  arkScore: number | null;
};

export type ArkHighlight = {
  category: string;
  allianceValue: number | null;
  playerName: string | null;
  playerValue: number | null;
};

export type ArkReportCoverage = {
  status: "available" | "missing" | "ambiguous";
  total: number;
};

export type ArkReportsResponse = {
  coverage: ArkReportCoverage;
  page: number;
  pageSize: number;
  items: ReportsListItem[];
};
