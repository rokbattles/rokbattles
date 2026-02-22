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
  } | null;
  results?: {
    total_score?: unknown;
    win_rate?: unknown;
    battles_win?: unknown;
  } | null;
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
