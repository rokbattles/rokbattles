import {
  ApplicationCommandOptionType,
  ApplicationCommandType,
  ContainerBuilder,
  MessageFlags,
  SeparatorBuilder,
} from "discord.js";
import type { Db, Document } from "mongodb";
import type { BaseClient } from "@/lib/base-client";
import type { CommandHandler } from "@/lib/command-handler";
import { getCommanderName } from "@/lib/get-commander-name";
import { mongo } from "@/lib/mongo";

interface ClaimedGovernorDocument {
  governorId: number;
}

interface BattleReportDocument {
  report?: {
    metadata?: {
      email_time?: unknown;
      start_date?: unknown;
      end_date?: unknown;
    };
    self?: {
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
}

interface MarchTotals {
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
}

interface AggregationBucket {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  count: number;
  totals: MarchTotals;
}

interface PairingAggregate {
  primaryCommanderId: number;
  secondaryCommanderId: number;
  count: number;
  totals: MarchTotals;
}

const numberFormatter = new Intl.NumberFormat("en-US");
const perSecondFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 1,
  minimumFractionDigits: 0,
});

function normalizeNumber(value: unknown, fallback = 0): number {
  const numeric = typeof value === "bigint" ? Number(value) : Number(value);
  return Number.isFinite(numeric) ? numeric : fallback;
}

function createEmptyTotals(): MarchTotals {
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

function createMatchStage(
  governorId: number,
  startMillis: number,
  endMillis: number
): Document {
  const startSeconds = Math.floor(startMillis / 1000);
  const endSeconds = Math.floor(endMillis / 1000);
  const startMicros = Math.floor(startMillis * 1000);
  const endMicros = Math.floor(endMillis * 1000);

  return {
    "report.self.player_id": governorId,
    "report.enemy.player_id": { $nin: [-2, 0] },
    $or: [
      {
        "report.metadata.start_date": {
          $gte: startSeconds,
          $lt: endSeconds,
        },
      },
      {
        "report.metadata.start_date": { $exists: false },
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
      },
    ],
  } satisfies Document;
}

function normalizeTimestampMillis(value: unknown): number | null {
  const numeric = normalizeNumber(value, Number.NaN);
  if (!Number.isFinite(numeric) || numeric <= 0) {
    return null;
  }

  if (numeric >= 1e14) {
    return numeric / 1000;
  }

  if (numeric < 1e12) {
    return numeric * 1000;
  }

  return numeric;
}

function extractEventTimeMillis(
  report: BattleReportDocument["report"]
): number | null {
  const rawMetadata = report?.metadata;
  if (!rawMetadata) {
    return null;
  }

  const emailTime = normalizeTimestampMillis(rawMetadata.email_time);
  if (emailTime != null) {
    return emailTime;
  }

  const startDate = normalizeTimestampMillis(rawMetadata.start_date);
  if (startDate != null) {
    return startDate;
  }

  return null;
}

function extractBattleDurationMillis(
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

  return Math.max(0, end - start);
}

function aggregateReports(
  reports: BattleReportDocument[],
  startMillis: number,
  endMillis: number
) {
  const buckets = new Map<string, AggregationBucket>();

  for (const doc of reports) {
    const report = doc.report;
    if (!report) {
      continue;
    }

    const eventTime = extractEventTimeMillis(report);
    if (
      eventTime == null ||
      eventTime < startMillis ||
      eventTime >= endMillis
    ) {
      continue;
    }

    const primaryCommanderId = Math.trunc(
      normalizeNumber(report.self?.primary_commander?.id)
    );
    if (primaryCommanderId <= 0) {
      continue;
    }

    const secondaryCommanderId = Math.trunc(
      normalizeNumber(report.self?.secondary_commander?.id)
    );
    const key = `${primaryCommanderId}:${secondaryCommanderId}`;

    let bucket = buckets.get(key);
    if (!bucket) {
      bucket = {
        primaryCommanderId,
        secondaryCommanderId,
        count: 0,
        totals: createEmptyTotals(),
      };
      buckets.set(key, bucket);
    }

    const battleResults = report.battle_results;
    bucket.count += 1;

    if (battleResults) {
      const killScore = normalizeNumber(battleResults.kill_score);
      const deaths = normalizeNumber(battleResults.death);
      const severelyWounded = normalizeNumber(battleResults.severely_wounded);
      const wounded = normalizeNumber(battleResults.wounded);
      const enemyKillScore = normalizeNumber(battleResults.enemy_kill_score);
      const enemyDeaths = normalizeNumber(battleResults.enemy_death);
      const enemySeverelyWounded = normalizeNumber(
        battleResults.enemy_severely_wounded
      );
      const enemyWounded = normalizeNumber(battleResults.enemy_wounded);

      bucket.totals.killScore += killScore;
      bucket.totals.deaths += deaths;
      bucket.totals.severelyWounded += severelyWounded;
      bucket.totals.wounded += wounded;
      bucket.totals.enemyKillScore += enemyKillScore;
      bucket.totals.enemyDeaths += enemyDeaths;
      bucket.totals.enemySeverelyWounded += enemySeverelyWounded;
      bucket.totals.enemyWounded += enemyWounded;
      bucket.totals.dps += enemyWounded + enemySeverelyWounded;
      bucket.totals.sps += enemySeverelyWounded;
      bucket.totals.tps += severelyWounded;
    }

    const battleDurationMillis = extractBattleDurationMillis(report);
    bucket.totals.battleDuration += battleDurationMillis;
  }

  return buckets;
}

function createPairing(primary: number, secondary?: number) {
  const primaryName = getCommanderName(primary);

  if (secondary && secondary > 0) {
    const secondaryName = getCommanderName(secondary);

    return `${primaryName ?? "Unknown"} / ${secondaryName ?? "Unknown"}`;
  }

  return primaryName ?? "Unknown";
}

function formatNumber(value: number): string {
  if (!Number.isFinite(value)) {
    return "0";
  }

  return numberFormatter.format(Math.round(value));
}

function formatDurationSeconds(value: number): string {
  if (!Number.isFinite(value) || value <= 0) {
    return "0s";
  }

  const units: [string, number][] = [
    ["d", 86_400],
    ["h", 3600],
    ["m", 60],
    ["s", 1],
  ];

  let remaining = Math.floor(value);
  const parts: string[] = [];

  for (const [label, seconds] of units) {
    if (remaining >= seconds) {
      const amount = Math.floor(remaining / seconds);
      parts.push(`${amount}${label}`);
      remaining -= amount * seconds;
    }
  }

  return parts.length ? parts.join(" ") : "0s";
}

function formatPerSecond(value: number): string {
  if (!Number.isFinite(value) || value <= 0) {
    return "0/s";
  }

  return `${perSecondFormatter.format(value)}/s`;
}

async function fetchGovernorPairingsForYear(
  db: Db,
  governorId: number
): Promise<PairingAggregate[]> {
  const startMillis = Date.UTC(2025, 0, 1);
  const endMillis = Date.UTC(2026, 0, 1);

  const projection: Document = {
    _id: 0,
    "report.metadata.email_time": 1,
    "report.metadata.start_date": 1,
    "report.metadata.end_date": 1,
    "report.self.primary_commander.id": 1,
    "report.self.secondary_commander.id": 1,
    "report.battle_results.kill_score": 1,
    "report.battle_results.death": 1,
    "report.battle_results.severely_wounded": 1,
    "report.battle_results.wounded": 1,
    "report.battle_results.enemy_kill_score": 1,
    "report.battle_results.enemy_death": 1,
    "report.battle_results.enemy_severely_wounded": 1,
    "report.battle_results.enemy_wounded": 1,
  };

  const reports = await db
    .collection<BattleReportDocument>("battleReports")
    .find(createMatchStage(governorId, startMillis, endMillis), { projection })
    .toArray();

  const buckets = aggregateReports(reports, startMillis, endMillis);
  const items: PairingAggregate[] = [];

  for (const bucket of buckets.values()) {
    items.push({
      primaryCommanderId: bucket.primaryCommanderId,
      secondaryCommanderId: bucket.secondaryCommanderId,
      count: bucket.count,
      totals: bucket.totals,
    });
  }

  items.sort((a, b) => {
    if (b.totals.killScore !== a.totals.killScore) {
      return b.totals.killScore - a.totals.killScore;
    }
    return b.count - a.count;
  });

  return items;
}

export const PairingsCommand: CommandHandler<BaseClient> = {
  options: {
    name: "pairings",
    description: "See statistics on your pairings",
    type: ApplicationCommandType.ChatInput,
    options: [
      {
        name: "governor",
        description: "Select a governor",
        type: ApplicationCommandOptionType.String,
        required: true,
        autocomplete: true,
      },
    ],
  },
  async chatInput(_client, interaction) {
    const discordId = interaction.user.id;
    const responseContainer = new ContainerBuilder();

    await interaction.deferReply();

    const client = await mongo();
    const db = client.db();

    const claimedGovernors = await db
      .collection<ClaimedGovernorDocument>("claimedGovernors")
      .find({ discordId })
      .sort({ createdAt: 1 })
      .toArray();

    if (claimedGovernors.length === 0) {
      responseContainer.addTextDisplayComponents((builder) =>
        builder.setContent(
          "It looks like you haven't claimed a governor yet. Log into the [ROK Battles platform](https://platform.rokbattles.com), claim a governor from the top-left menu, and retry the command."
        )
      );

      await interaction.editReply({
        components: [responseContainer],
        flags: MessageFlags.IsComponentsV2,
      });

      return;
    }

    const rawGovernorId = interaction.options.getString("governor", true);
    const governorId = Number.parseInt(rawGovernorId, 10);

    const isClaimed = claimedGovernors.some(
      (claim) => claim.governorId === governorId
    );
    if (!isClaimed) {
      responseContainer.addTextDisplayComponents((builder) =>
        builder.setContent(
          "You can only query pairings for a governor you have claimed."
        )
      );

      await interaction.editReply({
        components: [responseContainer],
        flags: MessageFlags.IsComponentsV2,
      });
      return;
    }

    const pairings = await fetchGovernorPairingsForYear(db, governorId);
    if (pairings.length === 0) {
      responseContainer.addTextDisplayComponents((builder) =>
        builder.setContent(`No pairings found for governor ${governorId}.`)
      );

      await interaction.editReply({
        components: [responseContainer],
        flags: MessageFlags.IsComponentsV2,
      });
      return;
    }

    const topPairings = pairings.slice(0, 5);
    responseContainer.addTextDisplayComponents((builder) =>
      builder.setContent("## Your Top 5 Pairings")
    );

    for (const [index, pairing] of topPairings.entries()) {
      const pairingName = createPairing(
        pairing.primaryCommanderId,
        pairing.secondaryCommanderId
      );
      const totalDurationSeconds =
        pairing.totals.battleDuration > 0
          ? pairing.totals.battleDuration / 1000
          : 0;
      const averageDurationSeconds =
        pairing.count > 0 ? totalDurationSeconds / pairing.count : 0;
      const dps =
        totalDurationSeconds > 0
          ? pairing.totals.dps / totalDurationSeconds
          : 0;
      const sps =
        totalDurationSeconds > 0
          ? pairing.totals.sps / totalDurationSeconds
          : 0;
      const tps =
        totalDurationSeconds > 0
          ? pairing.totals.tps / totalDurationSeconds
          : 0;

      const content = [
        `### ${index + 1}. ${pairingName}`,
        `Battles: ${formatNumber(pairing.count)}`,
        `Duration (Avg): ${formatDurationSeconds(averageDurationSeconds)}`,
        "",
        "Kill Points",
        `* You: ${formatNumber(pairing.totals.killScore)}`,
        `* Enemy: ${formatNumber(pairing.totals.enemyKillScore)}`,
        "",
        "Severely Wounded",
        `* Inflicted: ${formatNumber(pairing.totals.enemySeverelyWounded)}`,
        `* Taken: ${formatNumber(pairing.totals.severelyWounded)}`,
        "",
        "Rates",
        `* DPS: ${formatPerSecond(dps)}`,
        `* SPS: ${formatPerSecond(sps)}`,
        `* TPS: ${formatPerSecond(tps)}`,
      ].join("\n");

      responseContainer.addTextDisplayComponents((builder) =>
        builder.setContent(content)
      );

      if (index < topPairings.length - 1) {
        responseContainer.addSeparatorComponents(new SeparatorBuilder());
      }
    }

    await interaction.editReply({
      components: [responseContainer],
      flags: MessageFlags.IsComponentsV2,
    });
  },
  async autocomplete(_client, interaction) {
    const discordId = interaction.user.id;
    const focused = interaction.options.getFocused(true);

    if (focused.name === "governor") {
      const client = await mongo();
      const db = client.db();

      const claimedGovernors = await db
        .collection<ClaimedGovernorDocument>("claimedGovernors")
        .find({ discordId })
        .sort({ createdAt: 1 })
        .toArray();

      const query = focused.value.toLowerCase();
      const options = claimedGovernors
        .map((claim) => String(claim.governorId))
        .filter((value) => value.includes(query))
        .slice(0, 25)
        .map((value) => ({ name: value, value }));

      await interaction.respond(options);
      return;
    }

    await interaction.respond([]);
  },
};
