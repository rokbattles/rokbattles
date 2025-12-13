import type { Client } from "discord.js";
import type { EventHandler } from "@/lib/EventHandler";

export const SlashCommandHandler: EventHandler<Client, "interactionCreate"> = async (
  _client,
  interaction
) => {
  if (!interaction.isCommand() && !interaction.isAutocomplete()) return;

  //
};
