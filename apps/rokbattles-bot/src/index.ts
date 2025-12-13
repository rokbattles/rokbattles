import { BaseClient } from "@/lib/BaseClient";

const client = new BaseClient({ intents: [] });

(async () => {
  try {
    await client.login(process.env.DISCORD_TOKEN);
  } catch {
    await client.destroy();
  }
})();
