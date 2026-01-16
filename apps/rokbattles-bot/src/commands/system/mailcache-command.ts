import {
  ApplicationCommandOptionType,
  ApplicationCommandType,
  ContainerBuilder,
  MessageFlags,
} from "discord.js";
import type { BaseClient } from "@/lib/base-client";
import type { CommandHandler } from "@/lib/command-handler";

export const MailcacheCommand: CommandHandler<BaseClient> = {
  options: {
    name: "mailcache",
    description: "Guide to locate your Rise of Kingdoms mailcache directory",
    type: ApplicationCommandType.ChatInput,
    options: [
      {
        type: ApplicationCommandOptionType.Subcommand,
        name: "windows",
        description: "Find your mailcache directory on Windows",
      },
      {
        type: ApplicationCommandOptionType.Subcommand,
        name: "macos",
        description: "Find your mailcache directory on macOS",
      },
    ],
  },
  async chatInput(_client, interaction) {
    const subcommand = interaction.options.getSubcommand();
    const guideContainer = new ContainerBuilder();

    if (subcommand.toLowerCase() === "windows") {
      // Windows
      guideContainer.addTextDisplayComponents((builder) =>
        builder.setContent(
          [
            "## Mailcache Location (Windows)",
            "If you haven't changed the default install location of Rise of Kingdoms, the mailcache folder is usually here:",
            "`C:\\Program Files (x86)\\Rise of Kingdoms\\Rise of Kingdoms Game\\save\\mailcache`",
            "### What if it's not there?",
            "You can find your current installation path using the Rise of Kingdoms launcher:",
            "",
            "1. Open the Rise of Kingdoms launcher (do not launch the game)",
            "2. Click the ⚙️ icon in the top-right corner",
            '3. Select "Game Resources" from the sidebar',
            '4. Under "Local Files," find "Current installation path"',
            "",
            "From that base directory, navigate to:",
            "`Rise of Kingdoms Game\\save\\mailcache`",
          ].join("\n")
        )
      );
    } else {
      // macOS
      guideContainer.addTextDisplayComponents((builder) =>
        builder.setContent(
          [
            "## Mailcache Location (macOS)",
            "_Tip: You may need to show hidden directories in Finder with `Cmd + Shift + .`_",
            "",
            "1. Open Finder",
            "2. Go to the `/Library/Containers` directory",
            '3. Find the Rise of Kingdoms container directory called "RiseOfKingdoms"',
            "4. Open that directory, then navigate to: `Data/Documents/mailcache`",
            "",
            "_Note: Inside of ROK Battles desktop app, it will display the App ID instead of RiseOfKingdoms._",
          ].join("\n")
        )
      );
    }

    await interaction.reply({
      components: [guideContainer],
      flags: MessageFlags.IsComponentsV2,
    });
  },
};
