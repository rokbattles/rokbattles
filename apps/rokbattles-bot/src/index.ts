import { Client } from "discord.js";
import { SlashCommandHandler } from "@/events/SlashCommandHandler";
import { EventCollection, registerEvents } from "@/lib/EventHandler";

const client = new Client({ intents: [] });

function events(): EventCollection {
  const coll = new EventCollection();

  coll.add("interactionCreate", SlashCommandHandler);

  return coll;
}

(async () => {
  try {
    registerEvents(client, events());

    // Login
    await client.login(process.env.DISCORD_TOKEN);
  } catch {
    await client.destroy();
  }
})();
