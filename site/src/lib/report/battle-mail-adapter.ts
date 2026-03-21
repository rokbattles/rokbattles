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
  npc?: { type: number | null; bType: number | null } | null
): RawParticipantInfo {
  const armamentFields = buildArmamentFields(player);

  return {
    player_id: toOptionalNumber(player.playerId),
    app_uid: player.appUid != null ? String(player.appUid) : undefined,
    player_name: player.playerName ?? undefined,
    alliance_tag: player.alliance.abbreviation ?? undefined,
    avatar_url: player.avatarUrl ?? undefined,
    frame_url: player.frameUrl ?? undefined,
    castle_x: toOptionalNumber(player.castle.x),
    castle_y: toOptionalNumber(player.castle.y),
    is_rally: player.rally ?? undefined,
    alliance_building: player.allianceBuildingId ?? undefined,
    npc_type: npc?.type ?? undefined,
    npc_btype: npc?.bType ?? undefined,
    tracking_key: player.trackingKey ?? undefined,
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
  const sender = opponent.battleResults.sender;
  const enemy = opponent.battleResults.opponent;

  return {
    power: toOptionalNumber(sender.power),
    acclaim: toOptionalNumber(sender.acclaim),
    reinforcements_join: toOptionalNumber(sender.reinforcementsJoin),
    reinforcements_retreat: toOptionalNumber(sender.reinforcementsLeave),
    skill_power: toOptionalNumber(sender.skillPower),
    attack_power: toOptionalNumber(sender.attackPower),
    init_max: toOptionalNumber(sender.troopUnitsMax),
    max: toOptionalNumber(sender.troopUnits),
    healing: toOptionalNumber(sender.heal),
    death: toOptionalNumber(sender.dead),
    severely_wounded: toOptionalNumber(sender.severelyWounded),
    wounded: toOptionalNumber(sender.slightlyWounded),
    remaining: toOptionalNumber(sender.remaining),
    kill_score: toOptionalNumber(sender.killPoints),
    enemy_power: toOptionalNumber(enemy.power),
    enemy_acclaim: toOptionalNumber(enemy.acclaim),
    enemy_reinforcements_join: toOptionalNumber(enemy.reinforcementsJoin),
    enemy_reinforcements_retreat: toOptionalNumber(enemy.reinforcementsLeave),
    enemy_skill_power: toOptionalNumber(enemy.skillPower),
    enemy_attack_power: toOptionalNumber(enemy.attackPower),
    enemy_init_max: toOptionalNumber(enemy.troopUnitsMax),
    enemy_max: toOptionalNumber(enemy.troopUnits),
    enemy_healing: toOptionalNumber(enemy.heal),
    enemy_death: toOptionalNumber(enemy.dead),
    enemy_severely_wounded: toOptionalNumber(enemy.severelyWounded),
    enemy_wounded: toOptionalNumber(enemy.slightlyWounded),
    enemy_remaining: toOptionalNumber(enemy.remaining),
    enemy_kill_score: toOptionalNumber(enemy.killPoints),
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
  totals.troopUnits += toFiniteNumber(result?.troopUnits);
  totals.death += toFiniteNumber(result?.dead);
  totals.severelyWounded += toFiniteNumber(result?.severelyWounded);
  totals.wounded += toFiniteNumber(result?.slightlyWounded);
  totals.killPoints += toFiniteNumber(result?.killPoints);

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
    typeof senderSummary.severelyWounded === "number" &&
    typeof senderSummary.slightlyWounded === "number" &&
    typeof senderSummary.killPoints === "number" &&
    typeof senderSummary.troopUnits === "number" &&
    typeof senderSummary.remaining === "number" &&
    typeof opponentSummary.dead === "number" &&
    typeof opponentSummary.severelyWounded === "number" &&
    typeof opponentSummary.slightlyWounded === "number" &&
    typeof opponentSummary.killPoints === "number" &&
    typeof opponentSummary.troopUnits === "number" &&
    typeof opponentSummary.remaining === "number";

  if (hasSummaryMetrics) {
    return {
      max: senderSummary.troopUnits,
      death: senderSummary.dead,
      severely_wounded: senderSummary.severelyWounded,
      wounded: senderSummary.slightlyWounded,
      remaining: senderSummary.remaining,
      kill_score: senderSummary.killPoints,
      enemy_max: opponentSummary.troopUnits,
      enemy_death: opponentSummary.dead,
      enemy_severely_wounded: opponentSummary.severelyWounded,
      enemy_wounded: opponentSummary.slightlyWounded,
      enemy_remaining: opponentSummary.remaining,
      enemy_kill_score: opponentSummary.killPoints,
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
    aggregateResult(opponent.battleResults.sender, selfTotals);
    aggregateResult(opponent.battleResults.opponent, enemyTotals);
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
    (opponent) => !INVALID_OPPONENT_PLAYER_IDS.has(opponent.playerId)
  );

  return [...filtered].sort((a, b) => {
    const startTickDelta = toFiniteNumber(a.startTick) - toFiniteNumber(b.startTick);
    if (startTickDelta !== 0) {
      return startTickDelta;
    }
    return toFiniteNumber(a.playerId) - toFiniteNumber(b.playerId);
  });
}

function mapEntry(mail: BattleMail, opponent: BattleOpponent): ReportEntry {
  const timelineStartTimestamp = toFiniteNumber(mail.timeline.startTimestamp);
  const timelineStartTick = toFiniteNumber(mail.timeline.startTick);
  const startTick = toFiniteNumber(opponent.startTick);
  const endTickValue = toOptionalNumber(opponent.endTick);
  const endTick = endTickValue ?? startTick;
  const startTickOffset = startTick - timelineStartTick;
  const endTickOffset = endTick - timelineStartTick;
  const startDate = timelineStartTimestamp + startTickOffset;
  const endDate = timelineStartTimestamp + endTickOffset;

  const payload: RawReportPayload = {
    metadata: {
      email_id: mail.metadata.mailId,
      email_time: mail.metadata.mailTime,
      email_role: mail.metadata.mailRole,
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
