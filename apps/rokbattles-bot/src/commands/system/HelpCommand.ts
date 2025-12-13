import { ApplicationCommandType, MessageFlags, SectionBuilder } from "discord.js";
import type { BaseClient } from "@/lib/BaseClient";
import type { CommandHandler } from "@/lib/CommandHandler";

export const HelpCommand: CommandHandler<BaseClient> = {
  options: {
    name: "help",
    description: "Information about ROK Battles and relevant links",
    type: ApplicationCommandType.ChatInput,
  },
  async chatInput(_client, interaction, _args) {
    const helpSection = new SectionBuilder().addTextDisplayComponents(
      (builder) =>
        builder.setContent(
          "ROK Battles is a community-driven platform for sharing, exploring, and analyzing battle reports and viewing trends in Rise of Kingdoms."
        ),
      (builder) =>
        builder.setContent(
          [
            "### Relevant Links",
            "* Platform: <https://platform.rokbattles.com>",
            "* Desktop app: <https://platform.rokbattles.com/desktop-app>",
            "* Support: <https://platform.rokbattles.com/support>",
          ].join("\n")
        )
    );

    await interaction.reply({
      components: [helpSection],
      flags: MessageFlags.IsComponentsV2,
    });
  },
};
