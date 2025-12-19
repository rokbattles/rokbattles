import { ApplicationCommandType, ContainerBuilder, MessageFlags } from "discord.js";
import type { Db } from "mongodb";
import type { BaseClient } from "@/lib/BaseClient";
import type { CommandHandler } from "@/lib/CommandHandler";
import { getCommanderName } from "@/lib/getCommanderName";
import { mongo } from "@/lib/mongo";

type ClaimedGovernorDocument = {
  governorId: number;
};

type RecentReportDocument = {
  count: number;
  parentHash: string;
  self: {
    primary: number;
    secondary?: number;
  };
  enemy: {
    primary: number;
    secondary?: number;
  };
};

async function fetchRecentReports(db: Db, governorId: number) {
  return db
    .collection("battleReports")
    .aggregate<RecentReportDocument>([
      {
        $match: {
          // Filter out NPC & empty structures
          "report.enemy.player_id": { $nin: [-2, 0] },
          $or: [{ "report.self.player_id": governorId }, { "report.enemy.player_id": governorId }],
        },
      },
      {
        $group: {
          _id: "$metadata.parentHash",
          count: { $sum: 1 },
          firstStart: { $min: "$report.metadata.start_date" },
          latestMailTime: { $max: "$report.metadata.email_time" },
        },
      },
      { $sort: { latestMailTime: -1 } },
      { $limit: 5 },
      {
        $lookup: {
          from: "battleReports",
          let: { ph: "$_id", fs: "$firstStart" },
          pipeline: [
            {
              $match: {
                $expr: {
                  $and: [
                    { $eq: ["$metadata.parentHash", "$$ph"] },
                    { $eq: ["$report.metadata.start_date", "$$fs"] },
                  ],
                },
              },
            },
            { $sort: { "metadata.hash": 1 } },
            {
              $project: {
                selfPrimary: "$report.self.primary_commander.id",
                selfSecondary: { $ifNull: ["$report.self.secondary_commander.id", 0] },
                enemyPrimary: "$report.enemy.primary_commander.id",
                enemySecondary: { $ifNull: ["$report.enemy.secondary_commander.id", 0] },
              },
            },
            { $limit: 1 },
          ],
          as: "firstDoc",
        },
      },
      { $unwind: "$firstDoc" },
      {
        $project: {
          _id: 0,
          parentHash: "$_id",
          count: 1,
          self: { primary: "$firstDoc.selfPrimary", secondary: "$firstDoc.selfSecondary" },
          enemy: { primary: "$firstDoc.enemyPrimary", secondary: "$firstDoc.enemySecondary" },
        },
      },
    ])
    .toArray();
}

function createPairing(primary: number, secondary?: number) {
  const primaryName = getCommanderName(primary);

  if (secondary && secondary > 0) {
    const secondaryName = getCommanderName(secondary);

    return `${primaryName ?? "Unknown"}/${secondaryName ?? "Unknown"}`;
  }

  return primaryName ?? "Unknown";
}

export const ReportsCommand: CommandHandler<BaseClient> = {
  options: {
    name: "reports",
    description: "See your most recent battle reports",
    type: ApplicationCommandType.ChatInput,
  },
  async chatInput(_client, interaction) {
    const discordId = interaction.user.id;
    const responseContainer = new ContainerBuilder();

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

      await interaction.reply({
        components: [responseContainer],
        flags: MessageFlags.IsComponentsV2,
      });

      return;
    }

    const response: string[] = [];

    for (const claimedGovernor of claimedGovernors) {
      const reports = await fetchRecentReports(db, claimedGovernor.governorId);

      response.push(`### Recent battle reports for ${claimedGovernor.governorId}`);
      for (const report of reports) {
        const selfPairing = createPairing(report.self.primary, report.self.secondary);
        const enemyPairing = createPairing(report.enemy.primary, report.enemy.secondary);

        response.push(
          `* ${selfPairing} vs ${enemyPairing} (${report.count} battles) - [Full report](https://platform.rokbattles.com/report/${report.parentHash})`
        );
      }

      response.push("");
    }

    responseContainer.addTextDisplayComponents((builder) =>
      builder.setContent(response.join("\n"))
    );

    await interaction.reply({
      components: [responseContainer],
      flags: MessageFlags.IsComponentsV2,
    });
  },
};
