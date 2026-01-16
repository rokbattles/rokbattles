import { BaseClient } from "@/lib/base-client";

const client = new BaseClient({ intents: [] });

(async () => {
  try {
    await client.login(process.env.DISCORD_TOKEN);
  } catch {
    await client.destroy();
  }
})();
