import {
  ApplicationCommandType,
  ContainerBuilder,
  MessageFlags,
} from "discord.js";
import type { BaseClient } from "@/lib/base-client";
import type { CommandHandler } from "@/lib/command-handler";

export const HelpCommand: CommandHandler<BaseClient> = {
  options: {
    name: "help",
    description: "Information about ROK Battles and relevant links",
    type: ApplicationCommandType.ChatInput,
  },
  async chatInput(_client, interaction) {
    const helpContainer = new ContainerBuilder().addTextDisplayComponents(
      (builder) =>
        builder.setContent(
          [
            "## ROK Battles",
            "A community-driven platform for sharing, exploring, and analyzing battle reports and viewing trends in Rise of Kingdoms.",
            "### Relevant Links",
            "* Platform: <https://platform.rokbattles.com>",
            "* Desktop app: <https://platform.rokbattles.com/desktop-app>",
            "* Support: <https://platform.rokbattles.com/discord>",
          ].join("\n")
        )
    );

    await interaction.reply({
      components: [helpContainer],
      flags: MessageFlags.IsComponentsV2,
    });
  },
};
