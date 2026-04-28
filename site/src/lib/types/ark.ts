export type ArkMatchAlliance = {
  id: number | null;
  name: string | null;
  abbreviation: string | null;
  score: number | null;
  members: number | null;
  membersMax: number | null;
  isBlue: boolean | null;
};

export type ArkMatchRecord = {
  matchId: string;
  mailTimeMillis: number;
  battleResultsMailId: string | null;
  battleInfoMailId: string | null;
  individualResultsMailId: string | null;
  alliances: ArkMatchAlliance[];
  winnerAllianceId: number | null;
  hasBattleInfo: boolean;
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
  battles: number | null;
  killPointsGain: number | null;
  killPointsLoss: number | null;
};

export type ArkMatchDetailIndividualResults = {
  battlesWin: number | null;
  battlesLose: number | null;
  winRate: number | null;
  kills: number | null;
  severelyWounded: number | null;
  unitsHealed: number | null;
  speedups: number | null;
  teleports: number | null;
  structures: number | null;
  provisionsScore: number | null;
  arkOfOsirisScore: number | null;
  killScore: number | null;
  occupationScore: number | null;
};

export type ArkMatchDetailPairing = {
  primaryCommanderId: number | null;
  secondaryCommanderId: number | null;
  battles: number | null;
  battlesWin: number | null;
  killCount: number | null;
  killPoints: number | null;
  severelyWounded: number | null;
};

export type ArkMatchDetail = ArkMatchRecord & {
  overview: ArkMatchDetailOverview;
  individualResults: ArkMatchDetailIndividualResults;
  pairings: ArkMatchDetailPairing[];
};

export type ArkMatchDetailResponse = {
  id: string;
  match: ArkMatchDetail | null;
};
