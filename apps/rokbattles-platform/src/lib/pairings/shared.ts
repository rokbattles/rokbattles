import type { Document } from "mongodb";
import { normalizeTimestampMillis } from "@/lib/datetime";
import {
  type EquipmentToken,
  parseArmamentBuffs,
  parseEquipment,
  parseSemicolonNumberList,
} from "@/lib/report/parsers";

export type BattleReportDocument = {
  report?: {
    metadata?: {
      email_time?: unknown;
      start_date?: unknown;
      end_date?: unknown;
    };
    self?: {
      primary_commander?: { id?: unknown };
      secondary_commander?: { id?: unknown };
      equipment?: string;
      formation?: number;
      armament_buffs?: string;
      inscriptions?: string;
    };
    enemy?: {
      primary_commander?: { id?: unknown };
      secondary_commander?: { id?: unknown };
    };
    battle_results?: {
      kill_score?: unknown;
      death?: unknown;
      severely_wounded?: unknown;
      wounded?: unknown;
      enemy_kill_score?: unknown;
      enemy_death?: unknown;
      enemy_severely_wounded?: unknown;
      enemy_wounded?: unknown;
    };
  };
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

export function createMatchStage(
  governorId: number,
  startMillis: number,
  endMillis: number
) {
  const startMicros = Math.floor(startMillis * 1000);
  const endMicros = Math.floor(endMillis * 1000);

  return {
    "report.self.player_id": governorId,
    "report.enemy.player_id": { $nin: [-2, 0] },
    $or: [
      {
        "report.metadata.email_time": {
          $gte: startMillis,
          $lt: endMillis,
        },
      },
      {
        "report.metadata.email_time": {
          $gte: startMicros,
          $lt: endMicros,
        },
      },
    ],
  } satisfies Document;
}

export function extractEventTimeMillis(
  report: BattleReportDocument["report"]
): number | null {
  return normalizeTimestampMillis(report?.metadata?.email_time);
}

export function extractBattleDurationMillis(
  report: BattleReportDocument["report"]
): number {
  const rawMetadata = report?.metadata;
  if (!rawMetadata) {
    return 0;
  }

  const start = normalizeTimestampMillis(rawMetadata.start_date);
  const end = normalizeTimestampMillis(rawMetadata.end_date);

  if (start == null || end == null) {
    return 0;
  }

  return end - start;
}

export function applyBattleResults(
  totals: MarchTotals,
  battleResults: BattleReportDocument["report"] extends {
    battle_results?: infer Results;
  }
    ? Results
    : unknown
) {
  if (!battleResults) {
    return;
  }

  const killScore = Number(battleResults.kill_score);
  const deaths = Number(battleResults.death);
  const severelyWounded = Number(battleResults.severely_wounded);
  const wounded = Number(battleResults.wounded);
  const enemyKillScore = Number(battleResults.enemy_kill_score);
  const enemyDeaths = Number(battleResults.enemy_death);
  const enemySeverelyWounded = Number(battleResults.enemy_severely_wounded);
  const enemyWounded = Number(battleResults.enemy_wounded);

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

export function normalizeCommanderId(value: unknown) {
  const numeric = typeof value === "bigint" ? Number(value) : Number(value);
  return Number.isFinite(numeric) ? numeric : 0;
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

function normalizeArmaments(
  armaments: { id: number; value: number }[]
): LoadoutArmament[] {
  const ids = Array.from(new Set(armaments.map((buff) => buff.id))).sort(
    (a, b) => a - b
  );
  return ids.map((id) => ({ id }));
}

export function buildLoadoutSnapshot(
  report: BattleReportDocument["report"],
  granularity: LoadoutGranularity
): LoadoutSnapshot {
  const equipment = parseEquipment(report?.self?.equipment ?? null);
  const inscriptions = normalizeInscriptions(
    parseSemicolonNumberList(report?.self?.inscriptions ?? null)
  );
  const armaments = parseArmamentBuffs(report?.self?.armament_buffs ?? null);
  const formationValue = report?.self?.formation;
  const formation =
    typeof formationValue === "number" &&
    Number.isFinite(formationValue) &&
    formationValue !== 0
      ? formationValue
      : null;

  if (granularity === "normalized") {
    return {
      equipment: normalizeEquipmentTokens(equipment),
      inscriptions,
      armaments: normalizeArmaments(armaments),
      formation,
    };
  }

  return {
    equipment,
    inscriptions,
    armaments: armaments.map((buff) => ({ id: buff.id, value: buff.value })),
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
