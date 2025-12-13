import { ApplicationCommandType } from "discord.js";
import type { BaseClient } from "@/lib/BaseClient";
import type { EventHandler } from "@/lib/EventHandler";

export const SlashCommandHandler: EventHandler<BaseClient, "interactionCreate"> = async (
  client,
  interaction
) => {
  if (!interaction.isCommand() && !interaction.isAutocomplete()) return;

  const command = client.commands.get(interaction.commandName.toLowerCase());
  if (!command) return;

  switch (interaction.commandType) {
    case ApplicationCommandType.ChatInput: {
      if (interaction.isAutocomplete() && command.autocomplete) {
        await command.autocomplete(client, interaction, interaction.options.data);
        break;
      }

      if (interaction.isCommand()) {
        await command.chatInput(client, interaction, interaction.options.data);
        break;
      }

      break;
    }
    default:
      break;
  }
};
