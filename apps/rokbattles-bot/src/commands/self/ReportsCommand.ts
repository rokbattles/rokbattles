import { ApplicationCommandType, ContainerBuilder, MessageFlags } from "discord.js";
import type { BaseClient } from "@/lib/BaseClient";
import type { CommandHandler } from "@/lib/CommandHandler";
import { mongo } from "@/lib/mongo";

type ClaimedGovernorDocument = {
  governorId: number;
};

export const ReportsCommand: CommandHandler<BaseClient> = {
  options: {
    name: "reports",
    description: "See your most recent battle reports",
    type: ApplicationCommandType.ChatInput,
  },
  async chatInput(_client, interaction, _args) {
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

    responseContainer.addTextDisplayComponents((builder) =>
      builder.setContent("Not implemented yet.")
    );

    await interaction.reply({
      components: [responseContainer],
      flags: MessageFlags.IsComponentsV2,
    });
  },
};
