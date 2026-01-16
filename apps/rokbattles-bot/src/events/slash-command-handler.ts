import { ApplicationCommandType } from "discord.js";
import type { BaseClient } from "@/lib/base-client";
import type { EventHandler } from "@/lib/event-handler";

export const SlashCommandHandler: EventHandler<
  BaseClient,
  "interactionCreate"
> = async (client, interaction) => {
  if (!(interaction.isChatInputCommand() || interaction.isAutocomplete())) {
    return;
  }

  const command = client.commands.get(interaction.commandName.toLowerCase());
  if (!command) {
    return;
  }

  switch (interaction.commandType) {
    case ApplicationCommandType.ChatInput: {
      if (interaction.isAutocomplete() && command.autocomplete) {
        await command.autocomplete(client, interaction);
        break;
      }

      if (interaction.isChatInputCommand()) {
        await command.chatInput(client, interaction);
        break;
      }

      break;
    }
    default:
      break;
  }
};
