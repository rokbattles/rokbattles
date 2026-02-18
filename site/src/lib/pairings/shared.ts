import type { Document } from "mongodb";
import { normalizeTimestampMillis } from "@/lib/datetime";
import { type EquipmentToken, parseEquipment } from "@/lib/report/parsers";

const INVALID_OPPONENT_PLAYER_IDS = new Set([-2, 0]);

type PairingsBattleResultsSideDocument = {
  kill_points?: unknown;
  dead?: unknown;
  severely_wounded?: unknown;
  slightly_wounded?: unknown;
};

type PairingsBattleResultsDocument = {
  sender?: PairingsBattleResultsSideDocument | null;
  opponent?: PairingsBattleResultsSideDocument | null;
};

type PairingsCommanderDocument = {
  id?: unknown;
  formation?: unknown;
  equipment?: string | null;
  armaments?: PairingsArmamentDocument[] | null;
};

type PairingsArmamentDocument = {
  affix?: string | null;
  buffs?: string | null;
};

type PairingsOpponentDocument = {
  player_id?: unknown;
  start_tick?: unknown;
  end_tick?: unknown;
  commanders?: {
    primary?: { id?: unknown } | null;
    secondary?: { id?: unknown } | null;
  } | null;
  battle_results?: PairingsBattleResultsDocument | null;
};

export type PairingsBattleMailDocument = {
  metadata?: {
    mail_time?: unknown;
  };
  timeline?: {
    start_timestamp?: unknown;
  };
  sender?: {
    player_id?: unknown;
    commanders?: {
      primary?: PairingsCommanderDocument | null;
      secondary?: PairingsCommanderDocument | null;
    } | null;
  };
  opponents?: PairingsOpponentDocument[] | null;
};

export type PairingBattleEntry = {
  selfPrimaryCommanderId: number;
  selfSecondaryCommanderId: number;
  enemyPrimaryCommanderId: number;
  enemySecondaryCommanderId: number;
  battleDurationMillis: number;
  battleResults: PairingsBattleResultsDocument | null | undefined;
};

export type MarchTotals = {
  killScore: number;
  deaths: number;
  severelyWounded: number;
  wounded: number;
  enemyKillScore: number;
  enemyDeaths: number;
  enemySeverelyWounded: number;
  enemyWounded: number;
  dps: number;
  sps: number;
  tps: number;
  battleDuration: number;
};

export type LoadoutGranularity = "exact" | "normalized";

export type LoadoutArmament = {
  id: number;
  value?: number;
};

export type LoadoutSnapshot = {
  equipment: EquipmentToken[];
  armaments: LoadoutArmament[];
  inscriptions: number[];
  formation: number | null;
};

type DateRange = {
  startMillis: number;
  endMillis: number;
  year: number;
};

function toFiniteNumber(value: unknown): number {
  const numeric = typeof value === "bigint" ? Number(value) : Number(value);
  return Number.isFinite(numeric) ? numeric : 0;
}

export function createEmptyTotals(): MarchTotals {
  return {
    killScore: 0,
    deaths: 0,
    severelyWounded: 0,
    wounded: 0,
    enemyKillScore: 0,
    enemyDeaths: 0,
    enemySeverelyWounded: 0,
    enemyWounded: 0,
    dps: 0,
    sps: 0,
    tps: 0,
    battleDuration: 0,
  };
}

export function createMatchStage(governorId: number, startMillis: number, endMillis: number) {
  const startSeconds = Math.floor(startMillis / 1000);
  const endSeconds = Math.floor(endMillis / 1000);
  const startMicros = Math.floor(startMillis * 1000);
  const endMicros = Math.floor(endMillis * 1000);

  return {
    $and: [
      { "sender.player_id": governorId },
      { opponents: { $elemMatch: { player_id: { $nin: [-2, 0] } } } },
      {
        $or: [
          { "metadata.mail_time": { $gte: startSeconds, $lt: endSeconds } },
          { "metadata.mail_time": { $gte: startMillis, $lt: endMillis } },
          { "metadata.mail_time": { $gte: startMicros, $lt: endMicros } },
        ],
      },
    ],
  } satisfies Document;
}

export function extractEventTimeMillis(mail: PairingsBattleMailDocument): number | null {
  return normalizeTimestampMillis(mail.metadata?.mail_time);
}

export function normalizeCommanderId(value: unknown) {
  const numeric = typeof value === "bigint" ? Number(value) : Number(value);
  return Number.isFinite(numeric) ? numeric : 0;
}

export function extractBattleDurationMillis(
  mail: PairingsBattleMailDocument,
  opponent: PairingsOpponentDocument
): number {
  const timelineStart = toFiniteNumber(mail.timeline?.start_timestamp);
  const startTick = toFiniteNumber(opponent.start_tick);
  const endTickRaw = Number(opponent.end_tick);
  const endTick = Number.isFinite(endTickRaw) ? endTickRaw : startTick;
  const start = normalizeTimestampMillis(timelineStart + startTick);
  const end = normalizeTimestampMillis(timelineStart + endTick);

  if (start == null || end == null || end <= start) {
    return 0;
  }

  return end - start;
}

export function flattenPairingBattleEntries(
  mail: PairingsBattleMailDocument
): PairingBattleEntry[] {
  const selfPrimaryCommanderId = normalizeCommanderId(mail.sender?.commanders?.primary?.id);
  const selfSecondaryCommanderId = normalizeCommanderId(mail.sender?.commanders?.secondary?.id);
  const opponents = Array.isArray(mail.opponents) ? mail.opponents : [];

  const sortedOpponents = opponents
    .filter((opponent) => !INVALID_OPPONENT_PLAYER_IDS.has(toFiniteNumber(opponent.player_id)))
    .sort((a, b) => {
      const tickDelta = toFiniteNumber(a.start_tick) - toFiniteNumber(b.start_tick);
      if (tickDelta !== 0) {
        return tickDelta;
      }
      return toFiniteNumber(a.player_id) - toFiniteNumber(b.player_id);
    });

  return sortedOpponents.map((opponent) => ({
    selfPrimaryCommanderId,
    selfSecondaryCommanderId,
    enemyPrimaryCommanderId: normalizeCommanderId(opponent.commanders?.primary?.id),
    enemySecondaryCommanderId: normalizeCommanderId(opponent.commanders?.secondary?.id),
    battleDurationMillis: extractBattleDurationMillis(mail, opponent),
    battleResults: opponent.battle_results,
  }));
}

export function applyBattleResults(
  totals: MarchTotals,
  battleResults: PairingsBattleResultsDocument | null | undefined
) {
  if (!battleResults) {
    return;
  }

  const sender = battleResults.sender;
  const opponent = battleResults.opponent;
  const killScore = toFiniteNumber(sender?.kill_points);
  const deaths = toFiniteNumber(sender?.dead);
  const severelyWounded = toFiniteNumber(sender?.severely_wounded);
  const wounded = toFiniteNumber(sender?.slightly_wounded);
  const enemyKillScore = toFiniteNumber(opponent?.kill_points);
  const enemyDeaths = toFiniteNumber(opponent?.dead);
  const enemySeverelyWounded = toFiniteNumber(opponent?.severely_wounded);
  const enemyWounded = toFiniteNumber(opponent?.slightly_wounded);

  totals.killScore += killScore;
  totals.deaths += deaths;
  totals.severelyWounded += severelyWounded;
  totals.wounded += wounded;
  totals.enemyKillScore += enemyKillScore;
  totals.enemyDeaths += enemyDeaths;
  totals.enemySeverelyWounded += enemySeverelyWounded;
  totals.enemyWounded += enemyWounded;
  totals.dps += enemyWounded + enemySeverelyWounded;
  totals.sps += enemySeverelyWounded;
  totals.tps += severelyWounded;
}

function parseDateStart(value: string | null) {
  if (!value) {
    return null;
  }

  const parsed = new Date(`${value}T00:00:00Z`);
  const millis = parsed.getTime();
  return Number.isNaN(millis) ? null : millis;
}

function parseDateEnd(value: string | null) {
  if (!value) {
    return null;
  }

  const parsed = new Date(`${value}T23:59:59.999Z`);
  const millis = parsed.getTime();
  return Number.isNaN(millis) ? null : millis;
}

export function resolveDateRange(options: {
  startParam: string | null;
  endParam: string | null;
  fallbackYear: number;
}): DateRange {
  const { startParam, endParam, fallbackYear } = options;
  const startMillis = parseDateStart(startParam);
  const endMillisInclusive = parseDateEnd(endParam);

  if (startMillis != null && endMillisInclusive != null) {
    const endExclusive = endMillisInclusive + 1;
    if (endExclusive > startMillis) {
      return {
        startMillis,
        endMillis: endExclusive,
        year: new Date(startMillis).getUTCFullYear(),
      };
    }
  }

  return {
    startMillis: Date.UTC(fallbackYear, 0, 1, 0, 0, 0, 0),
    endMillis: Date.UTC(fallbackYear + 1, 0, 1, 0, 0, 0, 0),
    year: fallbackYear,
  };
}

function normalizeEquipmentAttr(attr: number | undefined) {
  if (!Number.isFinite(attr)) {
    return undefined;
  }

  const base = Math.trunc(attr / 10);
  return base > 0 ? base * 10 : 0;
}

function normalizeEquipmentTokens(tokens: EquipmentToken[]): EquipmentToken[] {
  return tokens.map((token) => ({
    ...token,
    attr: normalizeEquipmentAttr(token.attr),
  }));
}

function normalizeInscriptions(inscriptions: number[]) {
  return Array.from(new Set(inscriptions)).sort((a, b) => a - b);
}

function normalizeArmaments(armaments: { id: number; value: number }[]): LoadoutArmament[] {
  const ids = Array.from(new Set(armaments.map((buff) => buff.id))).sort((a, b) => a - b);
  return ids.map((id) => ({ id }));
}

function parseAffixIds(value: string | null | undefined): number[] {
  if (!value) {
    return [];
  }

  const matches = value.match(/-?\d+/g);
  if (!matches) {
    return [];
  }

  return matches.map((part) => Number(part)).filter((id) => Number.isFinite(id) && id > 0);
}

function parseBuffPairs(value: string | null | undefined): Array<{ id: number; value: number }> {
  if (!value) {
    return [];
  }

  const tokens = value
    .split(/[;,]/)
    .map((token) => token.trim())
    .filter(Boolean);
  const pairs: Array<{ id: number; value: number }> = [];

  for (const token of tokens) {
    const parts = token.split(/[_:]/).map((part) => part.trim());
    if (parts.length < 2) {
      continue;
    }

    const id = Number(parts[0]);
    const buffValue = Number(parts[1]);
    if (!Number.isFinite(id) || !Number.isFinite(buffValue) || id <= 0) {
      continue;
    }

    pairs.push({
      id,
      value: buffValue,
    });
  }

  return pairs;
}

export function buildLoadoutSnapshot(
  mail: PairingsBattleMailDocument,
  granularity: LoadoutGranularity
): LoadoutSnapshot {
  const sender = mail.sender;
  const equipment = parseEquipment(sender?.commanders?.primary?.equipment ?? null);
  const formationValue = toFiniteNumber(sender?.commanders?.primary?.formation);
  const formation = formationValue > 0 ? formationValue : null;

  const inscriptionSet = new Set<number>();
  const buffTotals = new Map<number, number>();
  const commanders = [sender?.commanders?.primary, sender?.commanders?.secondary];

  for (const commander of commanders) {
    const armaments = commander?.armaments ?? [];
    if (!Array.isArray(armaments)) {
      continue;
    }

    for (const armament of armaments) {
      const affixIds = parseAffixIds(armament.affix);
      for (const inscriptionId of affixIds) {
        inscriptionSet.add(inscriptionId);
      }

      const buffs = parseBuffPairs(armament.buffs);
      for (const buff of buffs) {
        buffTotals.set(buff.id, (buffTotals.get(buff.id) ?? 0) + buff.value);
      }
    }
  }

  const inscriptions = normalizeInscriptions(Array.from(inscriptionSet));
  const armamentBuffs = Array.from(buffTotals.entries())
    .sort((a, b) => a[0] - b[0])
    .map(([id, value]) => ({ id, value }));

  if (granularity === "normalized") {
    return {
      equipment: normalizeEquipmentTokens(equipment),
      inscriptions,
      armaments: normalizeArmaments(armamentBuffs),
      formation,
    };
  }

  return {
    equipment,
    inscriptions,
    armaments: armamentBuffs.map((buff) => ({ id: buff.id, value: buff.value })),
    formation,
  };
}

function serializeEquipment(tokens: EquipmentToken[]) {
  if (tokens.length === 0) {
    return "";
  }

  return tokens
    .map((token) => {
      const craft = token.craft ?? 0;
      const attr = token.attr ?? 0;
      return `${token.slot}:${token.id}_${craft}:${attr}`;
    })
    .join("|");
}

function serializeArmaments(armaments: LoadoutArmament[]) {
  if (armaments.length === 0) {
    return "";
  }

  return armaments
    .map((buff) => {
      if (buff.value == null) {
        return String(buff.id);
      }

      return `${buff.id}_${buff.value}`;
    })
    .join("|");
}

export function buildLoadoutKey(snapshot: LoadoutSnapshot) {
  const inscriptions = snapshot.inscriptions.join("|");
  const formation = snapshot.formation ?? "none";

  return [
    `eq:${serializeEquipment(snapshot.equipment)}`,
    `arm:${serializeArmaments(snapshot.armaments)}`,
    `ins:${inscriptions}`,
    `fm:${formation}`,
  ].join("|");
}
