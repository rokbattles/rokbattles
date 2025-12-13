import { unregisterGlobalCommands } from "@/lib/CommandHandler";

(async () => {
  try {
    // biome-ignore lint/style/noNonNullAssertion: ignore
    await unregisterGlobalCommands(process.env.DISCORD_TOKEN!, process.env.DISCORD_APPLICATION_ID!);
  } catch {
    process.exit(1);
  }
})();
