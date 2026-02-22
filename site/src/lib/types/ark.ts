export type ArkMailMetadataDocument = {
  mail_id?: unknown;
  mail_receiver?: unknown;
  mail_time?: unknown;
  server_id?: unknown;
};

export type ArkBattleResultsAllianceDocument = {
  alliance?: {
    id?: unknown;
    name?: unknown;
    abbreviation?: unknown;
  } | null;
  score?: unknown;
  members?: unknown;
  members_max?: unknown;
  is_blue?: unknown;
};

export type ArkBattleResultsMailDocument = {
  metadata?: ArkMailMetadataDocument | null;
  body?: {
    win?: unknown;
    alliance?: {
      id?: unknown;
    } | null;
  } | null;
  alliances?: ArkBattleResultsAllianceDocument[] | null;
};

export type ArkBattleInfoMailDocument = {
  metadata?: ArkMailMetadataDocument | null;
  body?: {
    win?: unknown;
    fights?: Array<{
      team?: unknown;
      time?: unknown;
      win?: unknown;
    }> | null;
  } | null;
};

export type ArkIndividualResultsMailDocument = {
  metadata?: ArkMailMetadataDocument | null;
  body?: {
    team?: unknown;
    win?: unknown;
  } | null;
  overview?: {
    player_id?: unknown;
    player_name?: unknown;
    rank?: unknown;
    score?: unknown;
    total_results?: {
      battles?: unknown;
      kill_points?: unknown;
      severely_wounded?: unknown;
    } | null;
  } | null;
  results?: {
    total_score?: unknown;
    win_rate?: unknown;
    battles_win?: unknown;
    battles_lose?: unknown;
    severely_wounded?: unknown;
    kills?: unknown;
    kill_score?: unknown;
    flag_score?: unknown;
    building_score?: unknown;
    gather_score?: unknown;
    units_healed?: unknown;
    speedups?: unknown;
    teleports?: unknown;
    structures?: unknown;
  } | null;
  pairings?: ArkIndividualResultsPairingDocument[] | null;
};

export type ArkIndividualResultsPairingDocument = {
  primary_commander?: {
    id?: unknown;
  } | null;
  secondary_commander?: {
    id?: unknown;
  } | null;
  battles?: unknown;
  battles_win?: unknown;
  kill_count?: unknown;
  kill_points?: unknown;
  severely_wounded?: unknown;
};

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
  rows: ArkMatchRecord[];
};

export type ArkQueryInput = {
  governorId: number;
  limit?: number;
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

export type ArkMatchDetailQueryInput = {
  governorId: number;
  matchId: string;
};
