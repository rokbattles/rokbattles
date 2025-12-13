import { Client } from "discord.js";

const client = new Client({ intents: [] });

(async () => {
  try {
    await client.login(process.env.DISCORD_TOKEN);
  } catch {
    await client.destroy();
  }
})();
