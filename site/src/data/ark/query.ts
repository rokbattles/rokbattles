import "server-only";

import { type MatchedArkMailSet, matchArkMails } from "@/data/ark/matcher";
import {
  fetchArkBattleInfoMails,
  fetchArkBattleResultsMailById,
  fetchArkBattleResultsMails,
  fetchArkIndividualResultsMails,
} from "@/data/ark/sources";
import { requireGovernorAccess } from "@/data/common/governor-access";
import { buildMailTimeMatch } from "@/data/common/mail-time-match";
import { normalizeTimestampMillis } from "@/lib/datetime";
import type {
  ArkIndividualResultsMailDocument,
  ArkMatchAlliance,
  ArkMatchDetail,
  ArkMatchDetailIndividualResults,
  ArkMatchDetailOverview,
  ArkMatchDetailPairing,
  ArkMatchDetailQueryInput,
  ArkMatchHistoryResult,
  ArkMatchRecord,
  ArkQueryInput,
} from "@/lib/types/ark";

const DEFAULT_LIMIT = 100;
const MAX_LIMIT = 250;
const MATCH_DELTA_MILLIS = 60_000;

function parseNumeric(value: unknown): number | null {
  if (value == null) {
    return null;
  }

  const parsed = typeof value === "bigint" ? Number(value) : Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function parseBoolean(value: unknown): boolean | null {
  if (typeof value === "boolean") {
    return value;
  }

  if (typeof value === "number") {
    if (value === 1) {
      return true;
    }
    if (value === 0) {
      return false;
    }
  }

  if (typeof value === "string") {
    const normalized = value.trim().toLowerCase();
    if (normalized === "true" || normalized === "1") {
      return true;
    }
    if (normalized === "false" || normalized === "0") {
      return false;
    }
  }

  return null;
}

function parseString(value: unknown): string | null {
  if (typeof value !== "string") {
    return null;
  }

  const trimmed = value.trim();
  return trimmed === "" ? null : trimmed;
}

function resolveLimit(value: number | undefined): number {
  if (!Number.isFinite(value)) {
    return DEFAULT_LIMIT;
  }

  return Math.max(1, Math.min(Math.floor(value), MAX_LIMIT));
}

function toArkAlliance(value: unknown): ArkMatchAlliance | null {
  if (!value || typeof value !== "object") {
    return null;
  }

  const entry = value as {
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

  return {
    id: parseNumeric(entry.alliance?.id),
    name: parseString(entry.alliance?.name),
    abbreviation: parseString(entry.alliance?.abbreviation),
    score: parseNumeric(entry.score),
    members: parseNumeric(entry.members),
    membersMax: parseNumeric(entry.members_max),
    isBlue: parseBoolean(entry.is_blue),
  };
}

function deriveWinnerAllianceId(options: {
  alliances: ArkMatchAlliance[];
  selfAllianceId: number | null;
  didWin: boolean | null;
}): number | null {
  const { alliances, selfAllianceId, didWin } = options;
  if (selfAllianceId == null || didWin == null) {
    return null;
  }

  if (didWin) {
    return selfAllianceId;
  }

  const opposingAlliance = alliances.find(
    (alliance) => alliance.id != null && alliance.id !== selfAllianceId
  );
  return opposingAlliance?.id ?? null;
}

function buildSecondaryWindow(
  mailTimes: number[]
): { startMillis: number; endMillis: number } | null {
  if (mailTimes.length === 0) {
    return null;
  }

  const minTime = Math.min(...mailTimes);
  const maxTime = Math.max(...mailTimes);
  return {
    startMillis: minTime - MATCH_DELTA_MILLIS,
    endMillis: maxTime + MATCH_DELTA_MILLIS + 1,
  };
}

function toArkMatchRecord(entry: MatchedArkMailSet, fallbackIndex: number): ArkMatchRecord {
  const alliances = (entry.battleResults.alliances ?? [])
    .map((alliance) => toArkAlliance(alliance))
    .filter((alliance): alliance is ArkMatchAlliance => alliance != null);
  const selfAllianceId = parseNumeric(entry.battleResults.body?.alliance?.id);
  const didWin = parseBoolean(entry.battleResults.body?.win);
  const winnerAllianceId = deriveWinnerAllianceId({ alliances, selfAllianceId, didWin });
  const matchId =
    entry.battleResultsMailId ?? `${entry.battleResultsTimeMillis}-${fallbackIndex + 1}`;

  return {
    matchId,
    mailTimeMillis: entry.battleResultsTimeMillis,
    battleResultsMailId: entry.battleResultsMailId,
    battleInfoMailId: entry.battleInfoMailId,
    individualResultsMailId: entry.individualResultsMailId,
    alliances,
    winnerAllianceId,
    hasBattleInfo: entry.battleInfo != null,
    hasIndividualResults: entry.individualResults != null,
  };
}

function toArkMatchDetailOverview(
  individualResults: ArkIndividualResultsMailDocument | null | undefined
): ArkMatchDetailOverview {
  const totals = individualResults?.overview?.total_results;
  return {
    rank: parseNumeric(individualResults?.overview?.rank),
    score: parseNumeric(individualResults?.overview?.score),
    battles: parseNumeric(totals?.battles),
    killPointsGain: parseNumeric(totals?.kill_points),
    killPointsLoss: parseNumeric(totals?.severely_wounded),
  };
}

function toArkMatchDetailIndividualResults(
  individualResults: ArkIndividualResultsMailDocument | null | undefined
): ArkMatchDetailIndividualResults {
  const results = individualResults?.results;
  return {
    battlesWin: parseNumeric(results?.battles_win),
    battlesLose: parseNumeric(results?.battles_lose),
    winRate: parseNumeric(results?.win_rate),
    kills: parseNumeric(results?.kills),
    severelyWounded: parseNumeric(results?.severely_wounded),
    unitsHealed: parseNumeric(results?.units_healed),
    speedups: parseNumeric(results?.speedups),
    teleports: parseNumeric(results?.teleports),
    structures: parseNumeric(results?.structures),
    provisionsScore: parseNumeric(results?.gather_score),
    arkOfOsirisScore: parseNumeric(results?.flag_score),
    killScore: parseNumeric(results?.kill_score),
    occupationScore: parseNumeric(results?.building_score),
  };
}

function toArkMatchDetailPairings(
  individualResults: ArkIndividualResultsMailDocument | null | undefined
): ArkMatchDetailPairing[] {
  const pairings = individualResults?.pairings ?? [];
  const rows: ArkMatchDetailPairing[] = [];

  for (const pairing of pairings) {
    if (!pairing || typeof pairing !== "object") {
      continue;
    }

    rows.push({
      primaryCommanderId: parseNumeric(pairing.primary_commander?.id),
      secondaryCommanderId: parseNumeric(pairing.secondary_commander?.id),
      battles: parseNumeric(pairing.battles),
      battlesWin: parseNumeric(pairing.battles_win),
      killCount: parseNumeric(pairing.kill_count),
      killPoints: parseNumeric(pairing.kill_points),
      severelyWounded: parseNumeric(pairing.severely_wounded),
    });
  }

  return rows;
}

export async function getGovernorArkMatchHistory(
  input: ArkQueryInput
): Promise<ArkMatchHistoryResult> {
  const { governorId } = input;
  const limit = resolveLimit(input.limit);
  const { db } = await requireGovernorAccess(governorId);
  const mailReceiver = `player_${governorId}`;

  const battleResults = await fetchArkBattleResultsMails(db, { mailReceiver, limit });
  if (battleResults.length === 0) {
    return {
      limit,
      total: 0,
      rows: [],
    };
  }

  const primaryTimes = battleResults
    .map((mail) => normalizeTimestampMillis(mail.metadata?.mail_time))
    .filter((mailTimeMillis): mailTimeMillis is number => mailTimeMillis != null);
  const secondaryWindow = buildSecondaryWindow(primaryTimes);

  let battleInfo = [] as Awaited<ReturnType<typeof fetchArkBattleInfoMails>>;
  let individualResults = [] as Awaited<ReturnType<typeof fetchArkIndividualResultsMails>>;

  if (secondaryWindow) {
    const timeMatch = buildMailTimeMatch(secondaryWindow.startMillis, secondaryWindow.endMillis);
    [battleInfo, individualResults] = await Promise.all([
      fetchArkBattleInfoMails(db, { mailReceiver, timeMatch }),
      fetchArkIndividualResultsMails(db, { mailReceiver, timeMatch }),
    ]);
  }

  const matched = matchArkMails({
    battleResults,
    battleInfo,
    individualResults,
    maxDeltaMillis: MATCH_DELTA_MILLIS,
  });

  const rows = matched.map((entry, index) => toArkMatchRecord(entry, index));

  return {
    limit,
    total: rows.length,
    rows,
  };
}

export async function getGovernorArkMatchDetail(
  input: ArkMatchDetailQueryInput
): Promise<ArkMatchDetail | null> {
  const { governorId } = input;
  const matchId = parseString(input.matchId);
  if (!matchId) {
    return null;
  }

  const { db } = await requireGovernorAccess(governorId);
  const mailReceiver = `player_${governorId}`;
  const battleResults = await fetchArkBattleResultsMailById(db, { mailReceiver, mailId: matchId });
  if (!battleResults) {
    return null;
  }

  const mailTimeMillis = normalizeTimestampMillis(battleResults.metadata?.mail_time);
  if (mailTimeMillis == null) {
    return null;
  }

  const timeMatch = buildMailTimeMatch(
    mailTimeMillis - MATCH_DELTA_MILLIS,
    mailTimeMillis + MATCH_DELTA_MILLIS + 1
  );
  const [battleInfo, individualResults] = await Promise.all([
    fetchArkBattleInfoMails(db, { mailReceiver, timeMatch }),
    fetchArkIndividualResultsMails(db, { mailReceiver, timeMatch }),
  ]);

  const [match] = matchArkMails({
    battleResults: [battleResults],
    battleInfo,
    individualResults,
    maxDeltaMillis: MATCH_DELTA_MILLIS,
  });

  if (!match) {
    return null;
  }

  const record = toArkMatchRecord(match, 0);
  return {
    ...record,
    overview: toArkMatchDetailOverview(match.individualResults),
    individualResults: toArkMatchDetailIndividualResults(match.individualResults),
    pairings: toArkMatchDetailPairings(match.individualResults),
  };
}
