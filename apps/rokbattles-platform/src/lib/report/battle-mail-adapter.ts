import type {
  BattleDetailedResult,
  BattleMail,
  BattleOpponent,
  BattlePlayer,
} from "@/lib/types/battle";
import type {
  RawBattleResults,
  RawOverview,
  RawParticipantInfo,
  RawReportPayload,
} from "@/lib/types/raw-report";
import type { ReportEntry } from "@/lib/types/report";

const INVALID_OPPONENT_PLAYER_IDS = new Set([-2, 0]);

type AdaptedBattleMailReport = {
  entries: ReportEntry[];
  overview: RawOverview | null;
  selfParticipant?: RawParticipantInfo;
  enemyParticipant?: RawParticipantInfo;
};

function toFiniteNumber(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

function toOptionalNumber(value: unknown): number | undefined {
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
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

    if (!Number.isFinite(id) || !Number.isFinite(buffValue)) {
      continue;
    }

    pairs.push({
      id,
      value: buffValue,
    });
  }

  return pairs;
}

function buildArmamentFields(player: BattlePlayer): {
  inscriptions?: string;
  armamentBuffs?: string;
} {
  const inscriptionSet = new Set<number>();
  const buffTotals = new Map<number, number>();
  const commanders = [player.commanders.primary, player.commanders.secondary];

  for (const commander of commanders) {
    const armaments = commander.armaments ?? [];

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

  const inscriptions = Array.from(inscriptionSet).sort((a, b) => a - b);
  const armamentBuffs = Array.from(buffTotals.entries())
    .sort((a, b) => a[0] - b[0])
    .map(([id, value]) => `${id}_${value}`)
    .join(";");

  return {
    inscriptions: inscriptions.length > 0 ? inscriptions.join(";") : undefined,
    armamentBuffs: armamentBuffs.length > 0 ? armamentBuffs : undefined,
  };
}

function mapPlayerToParticipant(
  player: BattlePlayer,
  npc?: { type: number | null; b_type: number | null } | null
): RawParticipantInfo {
  const armamentFields = buildArmamentFields(player);

  return {
    player_id: toOptionalNumber(player.player_id),
    app_uid: player.app_uid != null ? String(player.app_uid) : undefined,
    player_name: player.player_name ?? undefined,
    alliance_tag: player.alliance.abbreviation ?? undefined,
    avatar_url: player.avatar_url ?? undefined,
    frame_url: player.frame_url ?? undefined,
    castle_x: toOptionalNumber(player.castle.x),
    castle_y: toOptionalNumber(player.castle.y),
    is_rally: player.rally ?? undefined,
    alliance_building: player.alliance_building_id ?? undefined,
    npc_type: npc?.type ?? undefined,
    npc_btype: npc?.b_type ?? undefined,
    tracking_key: player.tracking_key ?? undefined,
    primary_commander: {
      id: player.commanders.primary.id ?? undefined,
      level: player.commanders.primary.level ?? undefined,
    },
    secondary_commander: {
      id: player.commanders.secondary.id ?? undefined,
      level: player.commanders.secondary.level ?? undefined,
    },
    formation: player.commanders.primary.formation ?? undefined,
    equipment: player.commanders.primary.equipment ?? undefined,
    equipment_2: player.commanders.secondary.equipment ?? undefined,
    armament_buffs: armamentFields.armamentBuffs,
    inscriptions: armamentFields.inscriptions,
  };
}

function mapBattleResultsForOpponent(opponent: BattleOpponent): RawBattleResults {
  const sender = opponent.battle_results.sender;
  const enemy = opponent.battle_results.opponent;

  return {
    power: toOptionalNumber(sender.power),
    acclaim: toOptionalNumber(sender.acclaim),
    reinforcements_join: toOptionalNumber(sender.reinforcements_join),
    reinforcements_retreat: toOptionalNumber(sender.reinforcements_leave),
    skill_power: toOptionalNumber(sender.skill_power),
    attack_power: toOptionalNumber(sender.attack_power),
    init_max: toOptionalNumber(sender.troop_units_max),
    max: toOptionalNumber(sender.troop_units),
    healing: toOptionalNumber(sender.heal),
    death: toOptionalNumber(sender.dead),
    severely_wounded: toOptionalNumber(sender.severely_wounded),
    wounded: toOptionalNumber(sender.slightly_wounded),
    remaining: toOptionalNumber(sender.remaining),
    watchtower: toOptionalNumber(sender.watchtower),
    watchtower_max: toOptionalNumber(sender.watchtower_max),
    kill_score: toOptionalNumber(sender.kill_points),
    enemy_power: toOptionalNumber(enemy.power),
    enemy_acclaim: toOptionalNumber(enemy.acclaim),
    enemy_reinforcements_join: toOptionalNumber(enemy.reinforcements_join),
    enemy_reinforcements_retreat: toOptionalNumber(enemy.reinforcements_leave),
    enemy_skill_power: toOptionalNumber(enemy.skill_power),
    enemy_attack_power: toOptionalNumber(enemy.attack_power),
    enemy_init_max: toOptionalNumber(enemy.troop_units_max),
    enemy_max: toOptionalNumber(enemy.troop_units),
    enemy_healing: toOptionalNumber(enemy.heal),
    enemy_death: toOptionalNumber(enemy.dead),
    enemy_severely_wounded: toOptionalNumber(enemy.severely_wounded),
    enemy_wounded: toOptionalNumber(enemy.slightly_wounded),
    enemy_remaining: toOptionalNumber(enemy.remaining),
    enemy_watchtower: toOptionalNumber(enemy.watchtower),
    enemy_watchtower_max: toOptionalNumber(enemy.watchtower_max),
    enemy_kill_score: toOptionalNumber(enemy.kill_points),
  };
}

function aggregateResult(
  result: BattleDetailedResult | null | undefined,
  totals: {
    troopUnits: number;
    death: number;
    severelyWounded: number;
    wounded: number;
    killPoints: number;
    remainingValues: number[];
  }
) {
  totals.troopUnits += toFiniteNumber(result?.troop_units);
  totals.death += toFiniteNumber(result?.dead);
  totals.severelyWounded += toFiniteNumber(result?.severely_wounded);
  totals.wounded += toFiniteNumber(result?.slightly_wounded);
  totals.killPoints += toFiniteNumber(result?.kill_points);

  const remaining = result?.remaining;
  if (typeof remaining === "number" && Number.isFinite(remaining)) {
    totals.remainingValues.push(remaining);
  }
}

function computeOverview(mail: BattleMail, opponents: BattleOpponent[]): RawOverview | null {
  const senderSummary = mail.summary.sender;
  const opponentSummary = mail.summary.opponent;
  const hasSummaryMetrics =
    senderSummary &&
    opponentSummary &&
    typeof senderSummary.dead === "number" &&
    typeof senderSummary.severely_wounded === "number" &&
    typeof senderSummary.slightly_wounded === "number" &&
    typeof senderSummary.kill_points === "number" &&
    typeof senderSummary.troop_units === "number" &&
    typeof senderSummary.remaining === "number" &&
    typeof opponentSummary.dead === "number" &&
    typeof opponentSummary.severely_wounded === "number" &&
    typeof opponentSummary.slightly_wounded === "number" &&
    typeof opponentSummary.kill_points === "number" &&
    typeof opponentSummary.troop_units === "number" &&
    typeof opponentSummary.remaining === "number";

  if (hasSummaryMetrics) {
    return {
      max: senderSummary.troop_units,
      death: senderSummary.dead,
      severely_wounded: senderSummary.severely_wounded,
      wounded: senderSummary.slightly_wounded,
      remaining: senderSummary.remaining,
      kill_score: senderSummary.kill_points,
      enemy_max: opponentSummary.troop_units,
      enemy_death: opponentSummary.dead,
      enemy_severely_wounded: opponentSummary.severely_wounded,
      enemy_wounded: opponentSummary.slightly_wounded,
      enemy_remaining: opponentSummary.remaining,
      enemy_kill_score: opponentSummary.kill_points,
    };
  }

  if (opponents.length === 0) {
    return null;
  }

  const selfTotals = {
    troopUnits: 0,
    death: 0,
    severelyWounded: 0,
    wounded: 0,
    killPoints: 0,
    remainingValues: [] as number[],
  };
  const enemyTotals = {
    troopUnits: 0,
    death: 0,
    severelyWounded: 0,
    wounded: 0,
    killPoints: 0,
    remainingValues: [] as number[],
  };

  for (const opponent of opponents) {
    aggregateResult(opponent.battle_results.sender, selfTotals);
    aggregateResult(opponent.battle_results.opponent, enemyTotals);
  }

  return {
    max: selfTotals.troopUnits,
    death: selfTotals.death,
    severely_wounded: selfTotals.severelyWounded,
    wounded: selfTotals.wounded,
    remaining: selfTotals.remainingValues.length > 0 ? Math.min(...selfTotals.remainingValues) : 0,
    kill_score: selfTotals.killPoints,
    enemy_max: enemyTotals.troopUnits,
    enemy_death: enemyTotals.death,
    enemy_severely_wounded: enemyTotals.severelyWounded,
    enemy_wounded: enemyTotals.wounded,
    enemy_remaining:
      enemyTotals.remainingValues.length > 0 ? Math.min(...enemyTotals.remainingValues) : 0,
    enemy_kill_score: enemyTotals.killPoints,
  };
}

function getValidSortedOpponents(mail: BattleMail): BattleOpponent[] {
  const filtered = mail.opponents.filter(
    (opponent) => !INVALID_OPPONENT_PLAYER_IDS.has(opponent.player_id)
  );

  return [...filtered].sort((a, b) => {
    const startTickDelta = toFiniteNumber(a.start_tick) - toFiniteNumber(b.start_tick);
    if (startTickDelta !== 0) {
      return startTickDelta;
    }
    return toFiniteNumber(a.player_id) - toFiniteNumber(b.player_id);
  });
}

function mapEntry(mail: BattleMail, opponent: BattleOpponent): ReportEntry {
  const timelineStart = toFiniteNumber(mail.timeline.start_timestamp);
  const startTick = toFiniteNumber(opponent.start_tick);
  const endTickValue = toOptionalNumber(opponent.end_tick);
  const endTick = endTickValue ?? startTick;
  const startDate = timelineStart + startTick;
  const endDate = timelineStart + endTick;

  const payload: RawReportPayload = {
    metadata: {
      email_id: mail.metadata.mail_id,
      email_time: mail.metadata.mail_time,
      email_role: mail.metadata.mail_role,
      is_kvk: mail.metadata.kvk ? 1 : 0,
      start_date: startDate,
      end_date: endDate,
      pos_x: opponent.attack.x,
      pos_y: opponent.attack.y,
    },
    self: mapPlayerToParticipant(mail.sender),
    enemy: mapPlayerToParticipant(opponent, opponent.npc),
    battle_results: mapBattleResultsForOpponent(opponent),
  };

  return {
    startDate,
    report: payload as Record<string, unknown>,
  };
}

export function adaptBattleMailToReport(mail: BattleMail): AdaptedBattleMailReport {
  const opponents = getValidSortedOpponents(mail);
  const entries = opponents.map((opponent) => mapEntry(mail, opponent));
  const overview = computeOverview(mail, opponents);

  return {
    entries,
    overview,
    selfParticipant: mapPlayerToParticipant(mail.sender),
    enemyParticipant: opponents[0]
      ? mapPlayerToParticipant(opponents[0], opponents[0].npc)
      : undefined,
  };
}
