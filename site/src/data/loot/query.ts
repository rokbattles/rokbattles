import "server-only";

import { requireGovernorAccess } from "@/data/common/governor-access";
import { buildMailTimeMatch } from "@/data/common/mail-time-match";
import {
  addLoot,
  addReport,
  createCategoryAggregate,
  toCategoryPayload,
} from "@/data/loot/category-aggregate";
import {
  extractEventTimeMillis,
  isBarbarian,
  parseNumeric,
  toDateKey,
} from "@/data/loot/normalizers";
import {
  fetchBarbarianBattleMails,
  fetchBarbarianFortMails,
  fetchBaulurMails,
} from "@/data/loot/sources";
import { resolveDateRange } from "@/lib/loot/date-range";
import type { LootQueryInput, LootQueryResult } from "@/lib/types/loot";

const LOOT_MAX_RANGE_DAYS = 366;

export async function getGovernorLootData(input: LootQueryInput): Promise<LootQueryResult> {
  const { governorId, startParam, endParam, yearParam } = input;
  const { db } = await requireGovernorAccess(governorId);

  const nowYear = new Date().getUTCFullYear();
  const parsedYear = yearParam ? Number(yearParam) : Number.NaN;
  const fallbackYear = Number.isFinite(parsedYear) ? parsedYear : nowYear;

  const range = resolveDateRange({
    startParam,
    endParam,
    fallbackYear,
    maxRangeDays: LOOT_MAX_RANGE_DAYS,
  });

  const mailReceiver = `player_${governorId}`;
  const timeMatch = buildMailTimeMatch(range.startMillis, range.endMillis);

  const [barbarianMails, barbarianFortMails, baulurMails] = await Promise.all([
    fetchBarbarianBattleMails(db, { mailReceiver, timeMatch }),
    fetchBarbarianFortMails(db, { mailReceiver, timeMatch }),
    fetchBaulurMails(db, { mailReceiver, timeMatch, governorId }),
  ]);

  const barbarian = createCategoryAggregate();
  const barbarianFort = createCategoryAggregate();
  const baulur = createCategoryAggregate();

  for (const mail of barbarianMails) {
    const eventTimeMillis = extractEventTimeMillis(mail.metadata?.mail_time);
    if (
      eventTimeMillis == null ||
      eventTimeMillis < range.startMillis ||
      eventTimeMillis >= range.endMillis
    ) {
      continue;
    }

    const opponents = Array.isArray(mail.opponents) ? mail.opponents : [];
    for (const opponent of opponents) {
      const opponentId = parseNumeric(opponent.player_id);
      if (opponentId !== -2) {
        continue;
      }

      const npcType = parseNumeric(opponent.npc?.type);
      const npcBType = parseNumeric(opponent.npc?.b_type);
      if (!isBarbarian(npcType, npcBType)) {
        continue;
      }

      const dateKey = toDateKey(eventTimeMillis);
      addReport(barbarian, dateKey);
      addLoot(barbarian, dateKey, opponent.npc?.loot);
    }
  }

  for (const mail of barbarianFortMails) {
    const eventTimeMillis = extractEventTimeMillis(mail.metadata?.mail_time);
    if (
      eventTimeMillis == null ||
      eventTimeMillis < range.startMillis ||
      eventTimeMillis >= range.endMillis
    ) {
      continue;
    }

    const dateKey = toDateKey(eventTimeMillis);
    addReport(barbarianFort, dateKey);
    addLoot(barbarianFort, dateKey, mail.rewards);
  }

  for (const mail of baulurMails) {
    const eventTimeMillis = extractEventTimeMillis(mail.metadata?.mail_time);
    if (
      eventTimeMillis == null ||
      eventTimeMillis < range.startMillis ||
      eventTimeMillis >= range.endMillis
    ) {
      continue;
    }

    const participants = Array.isArray(mail.participants) ? mail.participants : [];
    const matchingParticipants = participants.filter(
      (participant) => parseNumeric(participant.player_id) === governorId
    );
    if (matchingParticipants.length === 0) {
      continue;
    }

    const dateKey = toDateKey(eventTimeMillis);
    addReport(baulur, dateKey);

    for (const participant of matchingParticipants) {
      addLoot(baulur, dateKey, participant.loot);
    }
  }

  const categories = {
    barbarian: toCategoryPayload(barbarian),
    barbarianFort: toCategoryPayload(barbarianFort),
    baulur: toCategoryPayload(baulur),
  };

  return {
    year: range.year,
    range: {
      start: range.start,
      end: range.end,
    },
    totalReports:
      categories.barbarian.reports + categories.barbarianFort.reports + categories.baulur.reports,
    categories,
  };
}
